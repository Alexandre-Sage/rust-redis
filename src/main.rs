mod resp;

use resp::{serialize::serialize_simple_string, Resp};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Notify,
};

#[derive(Debug)]
struct RustRedis {
    port: i32,
    host: String,
}
impl Default for RustRedis {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 6379,
        }
    }
}

impl RustRedis {
    fn new(port: i32, host: String) -> Self {
        Self { port, host }
    }
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
    pub async fn run(&self, notify: Option<&Notify>) -> tokio::io::Result<()> {
        let addr = self.address();
        let listener = TcpListener::bind(&addr).await?;
        if let Some(notify) = notify {
            notify.notify_one();
        }
        log::info!("Rust redis is up");
        loop {
            match listener.accept().await {
                Ok((mut stream, _)) => {
                    tokio::spawn(async move {
                        log::info!("Incoming request");
                        loop {
                            let mut buf = vec![0u8; 1024];
                            match stream.read(&mut buf).await {
                                Ok(0) => {
                                    //let _ = stream.write_all(b"NO COMMAND WAS SENDED").await;
                                    break;
                                }
                                Ok(size) => {
                                    const PING_REGEX: &str = r"(?i)PING";
                                    let ping = regex::Regex::new(PING_REGEX)
                                        .expect("Cant create ping regex");
                                    let buf_as_str =
                                        std::str::from_utf8(&buf[..size]).expect("INVALID STRING");
                                    for _cap in ping.find_iter(buf_as_str) {
                                        let res = Resp::simple_string_from_str("PONG");
                                        stream.write_all(&res.serialize().unwrap()).await.unwrap();
                                    }
                                }
                                Err(_) => {
                                    break;
                                }
                            };
                        }
                    });
                }
                Err(err) => panic!("{:#?}", err),
            };
        }
    }
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    env_logger::init();
    let runner = RustRedis::new(6379, "127.0.0.1".to_string());
    runner.run(None).await
}
#[cfg(test)]
mod test {
    use std::{sync::Arc, usize};

    use super::*;
    use futures::future::join_all;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    };
    async fn setup() -> TcpStream {
        let notif = Arc::new(Notify::new());
        let notif2 = notif.clone();
        tokio::spawn(async move {
            let runner = RustRedis::default();
            let _ = runner.run(Some(&notif2)).await;
        });
        notif.notified().await;
        TcpStream::connect("127.0.0.1:6379").await.unwrap()
    }
    async fn send_request(mut stream: TcpStream, req: impl AsRef<[u8]>) -> Vec<u8> {
        stream.write_all(req.as_ref()).await.unwrap();
        let mut buf = vec![0u8; 1024];
        let size = stream.read(&mut buf).await.unwrap();
        buf.resize(size, 0u8);
        return buf;
    }
    #[tokio::test]
    async fn should_reply_to_ping() {
        const INPUT: &str = "PING";
        const EXPECT: &str = "+PONG\r\n";
        let stream = setup().await;
        let buf = send_request(stream, INPUT).await;
        assert_eq!(&buf, EXPECT.as_bytes())
    }

    #[tokio::test]
    async fn should_reply_to_multiple_ping() {
        const INPUT: &str = "PING\nPING";
        const EXPECT: &str = "+PONG";
        let stream = setup().await;
        let buf = send_request(stream, INPUT).await;
        let binding = String::from_utf8_lossy(&buf);
        let binding = binding.trim_end();
        let responses: Vec<&str> = binding.split("\r\n").collect();
        assert_eq!(responses.len(), 2);
        let ok = responses.iter().all(|pong| *pong == EXPECT);
        assert!(ok)
    }

    #[tokio::test]
    async fn should_handle_concurent_connection() {
        const INPUT: &str = "PING";
        const EXPECT: &str = "+PONG\r\n";
        const CLIENTS: usize = 6;
        tokio::spawn(async move {
            let runner = RustRedis::default();
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
            let (res, id) = result.unwrap();
            let test = String::from_utf8_lossy(&res);
            assert_eq!(test, EXPECT);
        }
    }
}
