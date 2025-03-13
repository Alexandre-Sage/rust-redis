pub mod commands;
mod data_management;
pub mod errors;
pub mod helpers;
mod resp;

use std::sync::Arc;

use commands::{
    command_registry::CommandRegistry,
    echo::{EchoCommand, ECHO_COMMAND_NAME},
    ping::PingCommand,
    set::{SetCommandHandler, SET_COMMAND_NAME},
};
use data_management::{message::DataChannelMessage, worker::data_management_worker_thread};
use errors::RustRedisError;
use resp::Resp;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{mpsc, Notify},
    task::JoinHandle,
};

#[derive(Debug)]
struct RustRedis {
    port: i32,
    host: String,
    command_registry: Arc<CommandRegistry>,
    data_management_worker: Option<JoinHandle<()>>,
}
impl Default for RustRedis {
    fn default() -> Self {
        let host = "127.0.0.1".to_string();
        let port = 6379;
        Self::new(port, host)
    }
}

impl RustRedis {
    fn new(port: i32, host: String) -> Self {
        let mut command_registry = CommandRegistry::new();
        let (data_sender, data_receiver) = mpsc::channel::<DataChannelMessage>(1000);
        command_registry.register(
            SET_COMMAND_NAME,
            Box::new(SetCommandHandler::new(data_sender)),
        );
        command_registry.register(ECHO_COMMAND_NAME, Box::new(EchoCommand::new()));
        command_registry.register("PING", Box::new(PingCommand));
        Self {
            port,
            host,
            command_registry: command_registry.into(),
            data_management_worker: Some(data_management_worker_thread(data_receiver, None)),
        }
    }
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub async fn run(&self, notify: Option<&Notify>) -> Result<(), RustRedisError> {
        let addr = self.address();
        let listener = TcpListener::bind(&addr).await?;
        if let Some(notify) = notify {
            notify.notify_one();
        }
        log::info!("Rust redis is up");
        loop {
            match listener.accept().await {
                Ok((mut stream, _)) => {
                    let command_registry = self.command_registry.clone();
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
                                    let commands = match parse_commands(&buf[..size]) {
                                        Ok(commands) => commands,
                                        Err(err) => {
                                            let err = Into::<Resp>::into(err);
                                            if let Ok(serialized) = err.serialize() {
                                                let _ = stream.write_all(&serialized).await;
                                            } else {
                                                log::error!("Unable to serialize error")
                                            }
                                            continue;
                                        }
                                    };
                                    for command in commands {
                                        match command {
                                            Resp::Array(command_with_args) => {
                                                if let Resp::BulkString(command) =
                                                    &command_with_args[0]
                                                {
                                                    let command = command_as_str(&command).unwrap();
                                                    let result = if command_with_args.len() == 1 {
                                                        command_registry
                                                            .no_args_command(command)
                                                            .await
                                                    } else {
                                                        command_registry
                                                            .command_with_args(
                                                                command,
                                                                &command_with_args[1..],
                                                            )
                                                            .await
                                                    }
                                                    .map_err(|err| Into::<Resp>::into(err));
                                                    let response = match result {
                                                        Ok(res) => res.serialize(),
                                                        Err(err) => err.serialize(),
                                                    };
                                                    if let Ok(response) = response {
                                                        match stream.write_all(&response).await {
                                                            Ok(_) => (),
                                                            Err(err) => {
                                                                log::error!("{}", err.to_string());
                                                                break;
                                                            }
                                                        }
                                                    } else if let Err(err) = response {
                                                        log::error!("{}", err.to_string());
                                                        break;
                                                    }
                                                }
                                            }
                                            _ => todo!(),
                                        }
                                    }
                                }
                                Err(err) => {
                                    log::error!("{:?}", err.to_string());
                                    break;
                                }
                            };
                        }
                    });
                }
                Err(err) => log::error!("{:?}", err.to_string()),
            };
        }
    }
}

pub fn parse_commands(input: &[u8]) -> Result<Vec<Resp>, RustRedisError> {
    let mut parsed = 0;
    let mut commands = Vec::new();
    while parsed < input.len() {
        let cur = Resp::deserialize(&input[parsed..])?;
        parsed += cur.size();
        commands.push(cur);
    }
    Ok(commands)
}

pub fn command_as_str(input: &[u8]) -> Result<&str, RustRedisError> {
    std::str::from_utf8(input).map_err(|err| RustRedisError::InvalidCommand(err.to_string()))
}

#[tokio::main]
async fn main() -> Result<(), RustRedisError> {
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
        let mut stream = setup().await;
        let buf = send_request(&mut stream, INPUT).await;
        assert_eq!(&buf, EXPECT.as_bytes())
    }

    #[tokio::test]
    async fn should_reply_to_multiple_ping() {
        // *2\r\n$4\r\nPING\r\n$4\r\nPING\r\n
        const INPUT: &str = "*1\r\n$4\r\nPING\r\n*1\r\n$4\r\nPING\r\n";
        const EXPECT: &str = "+PONG";
        let mut stream = setup().await;
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
            let (res, _id) = result.unwrap();
            let test = String::from_utf8_lossy(&res);
            assert_eq!(test, EXPECT);
        }
    }

    #[tokio::test]
    async fn should_handle_echo_command() {
        const EXPECT: &str = "$3\r\nhey\r\n";
        const INPUT: &str = "*2\r\n$4\r\nECHO\r\n$3\r\nhey\r\n";
        let mut stream = setup().await;
        let res = send_request(&mut stream, INPUT).await;
        let res = std::str::from_utf8(&res).unwrap();
        assert_eq!(res, EXPECT)
    }

    #[tokio::test]
    async fn should_reply_error_if_invalid_commands_parse() {
        const INPUT: &str = "\r\n$4\r\nECHO\r\n$3\r\nhey\r\n";
        const EXPECT: &str = "-ERR invalid resp prefix\r\n";
        let mut stream = setup().await;
        let res = send_request(&mut stream, INPUT).await;
        let res = std::str::from_utf8(&res).unwrap();
        assert_eq!(res, EXPECT)
    }

    #[tokio::test]
    async fn should_reply_ok_on_set_successfull() {
        const INPUT: &str = "*3\r\n$3\r\nSET\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
        const EXPECT: &str = "+OK\r\n";
        let mut stream = setup().await;
        let res = send_request(&mut stream, INPUT).await;
        let res = std::str::from_utf8(&res).unwrap();
        assert_eq!(res, EXPECT)
    }
    #[ignore = "not complete"]
    #[test]
    fn should_reply_error_if_invalid_utf8_command() {
        let test_cases = [
            &[0x80][..],                   // Standalone continuation byte
            &[0xC2][..],                   // Truncated 2-byte sequence
            &[0xE0, 0x80, 0x80][..],       // Overlong encoding
            &[0xED, 0xA0, 0x80][..],       // UTF-16 surrogate
            &[0xF0, 0x28, 0x8C, 0x28][..], // Invalid 4-byte sequence
            &[0x41, 0x42, 0xFF, 0x43][..], // Valid ASCII with invalid byte
        ];

        for input in test_cases {
            let res = command_as_str(input).unwrap_err();
            dbg!(&res);
            if let RustRedisError::InvalidCommand(err) = res {
                let ok = matches!(
                    err.as_str(),
                    "invalid utf-8 sequence of 1 bytes from index 0"
                        | "incomplete utf-8 byte sequence from index 0"
                );
                assert!(ok)
            } else {
                panic!()
            }
        }
    }
}
