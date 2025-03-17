use std::sync::Arc;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Notify,
};

use crate::{
    commands::{
        command_registry::CommandRegistry,
        echo::{EchoCommand, ECHO_COMMAND_NAME},
        get::{GetCommandHandler, GET_COMMAND_NAME},
        get_config::{GetConfigCommandHandler, GET_CONFIG_COMMAND_NAME},
        ping::PingCommand,
        set::{SetCommandHandler, SET_COMMAND_NAME},
    },
    config::AppConfig,
    data_management::message::DataChannelMessage,
    errors::RustRedisError,
    resp::Resp,
};

#[derive(Debug)]
pub struct EventLoop {
    port: i32,
    host: String,
    command_registry: Arc<CommandRegistry>,
    data_sender: Arc<tokio::sync::mpsc::Sender<DataChannelMessage>>,
}

impl EventLoop {
    pub fn new(
        port: i32,
        host: String,
        data_sender: Arc<tokio::sync::mpsc::Sender<DataChannelMessage>>,
        config: &AppConfig,
    ) -> Self {
        let mut command_registry = CommandRegistry::new();
        command_registry.register(
            GET_CONFIG_COMMAND_NAME,
            Box::new(GetConfigCommandHandler::new(config.into())),
        );
        command_registry.register(
            GET_COMMAND_NAME,
            Box::new(GetCommandHandler::new(data_sender.clone())),
        );
        command_registry.register(
            SET_COMMAND_NAME,
            Box::new(SetCommandHandler::new(data_sender.clone())),
        );
        command_registry.register(ECHO_COMMAND_NAME, Box::new(EchoCommand::new()));
        command_registry.register("PING", Box::new(PingCommand));

        Self {
            port,
            host,
            command_registry: command_registry.into(),
            data_sender,
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
