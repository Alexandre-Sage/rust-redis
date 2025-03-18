use std::{collections::HashMap, path::PathBuf};

use clap::{command, Parser};

use crate::{errors::AppError, resp::Resp};

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum ConfigField {
    Dbfilename,
    Dir,
}

impl TryFrom<&Resp> for ConfigField {
    type Error = AppError;
    fn try_from(value: &Resp) -> Result<Self, Self::Error> {
        if let Resp::BulkString(field) = value {
            match field.as_slice() {
                b"dbfilename" => Ok(Self::Dbfilename),
                b"dir" => Ok(Self::Dir),
                _ => Err(AppError::InvalidConfigField(
                    String::from_utf8_lossy(&field).to_string(),
                )),
            }
        } else {
            Err(AppError::InvalidArgType("bulk string".to_owned()))
        }
    }
}

#[derive(Parser, Debug, Default)]
#[command(version, about, long_about = None)]
pub struct AppConfig {
    #[arg(long)]
    pub dir: Option<PathBuf>,
    #[arg(long)]
    pub dbfilename: Option<String>,
}

impl From<&AppConfig> for HashMap<ConfigField, Resp> {
    fn from(value: &AppConfig) -> Self {
        let dir = value
            .dir
            .clone()
            .map(|dir| Resp::bulk_string_from_str(dir.to_str().unwrap()))
            .unwrap_or(Resp::null_bulk_string());

        let dbfilename = value
            .dbfilename
            .clone()
            .map(|dbfilename| Resp::bulk_string_from_str(&dbfilename))
            .unwrap_or(Resp::null_bulk_string());

        [
            (ConfigField::Dir, dir),
            (ConfigField::Dbfilename, dbfilename),
        ]
        .into()
    }
}
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn should_parse_dir_arg() {
        let args = AppConfig::try_parse_from(["config", "--dir", "/tmp/redis"]).unwrap();
        assert_eq!(args.dir.unwrap(), PathBuf::from_str("/tmp/redis").unwrap());
    }

    #[test]
    fn should_parse_dbfilename_arg() {
        let args = AppConfig::try_parse_from(["config", "--dbfilename", "redis.rdb"]).unwrap();
        assert_eq!(args.dbfilename.unwrap(), "redis.rdb");
    }
}
