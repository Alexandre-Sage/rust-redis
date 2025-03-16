pub mod commands;
mod data_management;
pub mod errors;
mod event_loop;
pub mod helpers;
mod resp;

use data_management::{
    datastore::DataStore, hash_table_store::HashTableDataStore, message::DataChannelMessage,
    worker::DataManager,
};
use errors::RustRedisError;
use event_loop::EventLoop;
use tokio::sync::{mpsc, Notify};

pub struct App<T>
where
    T: DataStore,
{
    event_loop: EventLoop,
    data_manager: DataManager<T>,
}

impl<T> App<T>
where
    T: DataStore,
{
    pub fn new(port: i32, host: String, data_store: Option<T>) -> Self {
        let (data_sender, data_receiver) = mpsc::channel::<DataChannelMessage>(1000);
        let event_loop = EventLoop::new(port, host, data_sender.into());
        let data_manager = DataManager::new(data_receiver, data_store, None);
        Self {
            event_loop,
            data_manager,
        }
    }

    pub async fn run(self, notif: Option<&Notify>) -> Result<(), RustRedisError> {
        self.data_manager.run();
        self.event_loop.run(notif).await
    }
}

#[tokio::main]
async fn main() -> Result<(), RustRedisError> {
    env_logger::init();
    let runner = App::<HashTableDataStore>::new(6379, "127.0.0.1".to_string(), None);
    runner.run(None).await
}
#[cfg(test)]
mod test {
    use std::{sync::Arc, time::Duration, usize};

    use super::*;
    use data_management::datastore::DataStoreEntry;
    use futures::future::join_all;
    use resp::Resp;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    };

    async fn setup(data: Option<HashTableDataStore>) -> TcpStream {
        let host = "127.0.0.1".to_string();
        let port = 6379;
        let notif = Arc::new(Notify::new());
        let notif2 = notif.clone();
        tokio::spawn(async move {
            let runner = App::new(port, host, data);
            let _ = runner.run(Some(&notif2)).await;
        });
        notif.notified().await;
        TcpStream::connect("127.0.0.1:6379").await.unwrap()
    }
    async fn send_request(stream: &mut TcpStream, req: impl AsRef<[u8]>) -> Vec<u8> {
        stream.write_all(req.as_ref()).await.unwrap();
        let mut buf = vec![0u8; 1024];
        let size = stream.read(&mut buf).await.unwrap();
        buf.resize(size, 0u8);
        return buf;
    }
    #[tokio::test]
    async fn should_reply_to_ping() {
        const INPUT: &str = "*1\r\n$4\r\nPING\r\n";
        const EXPECT: &str = "+PONG\r\n";
        let mut stream = setup(None).await;
        let buf = send_request(&mut stream, INPUT).await;
        assert_eq!(&buf, EXPECT.as_bytes())
    }

    #[tokio::test]
    async fn should_reply_to_multiple_ping() {
        // *2\r\n$4\r\nPING\r\n$4\r\nPING\r\n
        const INPUT: &str = "*1\r\n$4\r\nPING\r\n*1\r\n$4\r\nPING\r\n";
        const EXPECT: &str = "+PONG";
        let mut stream = setup(None).await;
        let buf = send_request(&mut stream, INPUT).await;
        let binding = String::from_utf8_lossy(&buf);
        let binding = binding.trim_end();
        let responses: Vec<&str> = binding.split("\r\n").collect();
        assert_eq!(responses.len(), 2);
        let ok = responses.iter().all(|pong| *pong == EXPECT);
        assert!(ok)
    }

    #[tokio::test]
    async fn should_handle_concurent_connection() {
        const INPUT: &str = "*1\r\n$4\r\nPING\r\n";
        const EXPECT: &str = "+PONG\r\n";
        const CLIENTS: usize = 6;
        tokio::spawn(async move {
            let runner = App::<HashTableDataStore>::new(6379, "0.0.0.0".to_owned(), None);
            let _ = runner.run(None).await;
        });
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let mut client_handles = Vec::with_capacity(CLIENTS);
        for client_id in 0..CLIENTS {
            let handle = tokio::spawn(async move {
                let mut stream = TcpStream::connect("127.0.0.1:6379").await.unwrap();
                stream.write_all(INPUT.as_bytes()).await.unwrap();
                let mut buf = Vec::with_capacity(1024);
                stream.shutdown().await.unwrap();
                let size = stream.read_to_end(&mut buf).await.unwrap();
                return (buf[..size].to_vec(), client_id);
            });
            client_handles.push(handle);
        }
        let results = join_all(client_handles).await;
        for result in results {
            let (res, _id) = result.unwrap();
            let test = String::from_utf8_lossy(&res);
            assert_eq!(test, EXPECT);
        }
    }

    #[tokio::test]
    async fn should_handle_echo_command() {
        const EXPECT: &str = "$3\r\nhey\r\n";
        const INPUT: &str = "*2\r\n$4\r\nECHO\r\n$3\r\nhey\r\n";
        let mut stream = setup(None).await;
        let res = send_request(&mut stream, INPUT).await;
        let res = std::str::from_utf8(&res).unwrap();
        assert_eq!(res, EXPECT)
    }

    #[tokio::test]
    async fn should_reply_error_if_invalid_commands_parse() {
        const INPUT: &str = "\r\n$4\r\nECHO\r\n$3\r\nhey\r\n";
        const EXPECT: &str = "-ERR invalid resp prefix\r\n";
        let mut stream = setup(None).await;
        let res = send_request(&mut stream, INPUT).await;
        let res = std::str::from_utf8(&res).unwrap();
        assert_eq!(res, EXPECT)
    }

    #[tokio::test]
    async fn should_reply_ok_on_set_successfull() {
        const INPUT: &str = "*3\r\n$3\r\nSET\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
        const EXPECT: &str = "+OK\r\n";
        let mut stream = setup(None).await;
        let res = send_request(&mut stream, INPUT).await;
        let res = std::str::from_utf8(&res).unwrap();
        assert_eq!(res, EXPECT)
    }

    #[tokio::test]
    async fn should_reply_to_get_command() {
        const INPUT: &str = "*2\r\n$3\r\nGET\r\n$5\r\nhello\r\n";
        const EXPECT: &str = "$5\r\nworld\r\n";

        let key = Resp::bulk_string_from_str("hello").serialize().unwrap();
        let value = Resp::bulk_string_from_str("world").serialize().unwrap();
        let entry = DataStoreEntry::new(value, None);
        let default = [(key.clone(), entry)];
        let mut stream = setup(Some(default.into())).await;

        let res = send_request(&mut stream, INPUT).await;
        let res = std::str::from_utf8(&res).unwrap();

        assert_eq!(res, EXPECT)
    }

    #[tokio::test]
    async fn shoudl_reply_null_bulk_string_if_no_data() {
        const INPUT: &str = "*2\r\n$3\r\nGET\r\n$5\r\nhello\r\n";
        const EXPECT: &str = "$-1\r\n";
        let mut stream = setup(None).await;

        let res = send_request(&mut stream, INPUT).await;
        let res = std::str::from_utf8(&res).unwrap();

        assert_eq!(res, EXPECT)
    }

    #[tokio::test]
    async fn should_set_data_with_expiry() {
        const INPUT: &str =
            "*5\r\n$3\r\nSET\r\n$5\r\nhello\r\n$5\r\nworld\r\n$2\r\nPX\r\n$1\r\n1\r\n";
        const EXPECT: &str = "+OK\r\n";
        let mut stream = setup(None).await;
        let res = send_request(&mut stream, INPUT).await;
        let res = std::str::from_utf8(&res).unwrap();
        assert_eq!(res, EXPECT)
    }

    #[tokio::test]
    async fn should_reply_null_bulk_string_if_expired_date() {
        const INPUT: &str = "*2\r\n$3\r\nGET\r\n$5\r\nhello\r\n";
        const EXPECT: &str = "$-1\r\n";

        let key = Resp::bulk_string_from_str("hello").serialize().unwrap();
        let value = Resp::bulk_string_from_str("world").serialize().unwrap();
        let expiry = Duration::from_millis(1);
        let entry = DataStoreEntry::new(value, Some(expiry));
        let default = [(key.clone(), entry)];

        let mut stream = setup(Some(default.into())).await;
        tokio::time::sleep(Duration::from_millis(2)).await;

        let res = send_request(&mut stream, INPUT).await;
        let res = std::str::from_utf8(&res).unwrap();

        assert_eq!(res, EXPECT)
    }
    //#[ignore = "not complete"]
    //#[test]
    //fn should_reply_error_if_invalid_utf8_command() {
    //    let test_cases = [
    //        &[0x80][..],                   // Standalone continuation byte
    //        &[0xC2][..],                   // Truncated 2-byte sequence
    //        &[0xE0, 0x80, 0x80][..],       // Overlong encoding
    //        &[0xED, 0xA0, 0x80][..],       // UTF-16 surrogate
    //        &[0xF0, 0x28, 0x8C, 0x28][..], // Invalid 4-byte sequence
    //        &[0x41, 0x42, 0xFF, 0x43][..], // Valid ASCII with invalid byte
    //    ];
    //
    //    for input in test_cases {
    //        let res = command_as_str(input).unwrap_err();
    //        dbg!(&res);
    //        if let RustRedisError::InvalidCommand(err) = res {
    //            let ok = matches!(
    //                err.as_str(),
    //                "invalid utf-8 sequence of 1 bytes from index 0"
    //                    | "incomplete utf-8 byte sequence from index 0"
    //            );
    //            assert!(ok)
    //        } else {
    //            panic!()
    //        }
    //    }
    //}
}
