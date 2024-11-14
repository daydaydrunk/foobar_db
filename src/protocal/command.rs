use crate::db::db::DB;
use crate::db::storage::Storage;
use crate::protocal::resp::RespValue;
use anyhow::{anyhow, Error};
use std::borrow::Cow;
use std::sync::Arc;

// 定义支持的命令类型
#[derive(Debug, PartialEq)]
pub enum Command {
    // 字符串操作
    Get {
        key: String,
    },
    Set {
        key: String,
        value: String,
    },
    Del {
        keys: Vec<String>,
    },

    // 列表操作
    LPush {
        key: String,
        values: Vec<String>,
    },
    RPush {
        key: String,
        values: Vec<String>,
    },
    LPop {
        key: String,
    },
    RPop {
        key: String,
    },

    // 集合操作
    SAdd {
        key: String,
        members: Vec<String>,
    },
    SRem {
        key: String,
        members: Vec<String>,
    },

    // 哈希表操作
    HSet {
        key: String,
        field: String,
        value: String,
    },
    HGet {
        key: String,
        field: String,
    },

    // 其他基础操作
    Ping,
    Echo {
        message: String,
    },

    // 未知或不支持的命令
    Unknown {
        command: String,
    },

    // 添加新的命令类型
    //todo
    Info,
    Command,
}

#[derive(Debug)]
pub enum CommandError {
    WrongNumberOfArguments { command: String },
    InvalidCommandName,
    EmptyCommand,
    InvalidArgumentType,
    NotImplemented,
    UnknownCommand(String),
    StorageError(Error),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongNumberOfArguments { command } => {
                write!(f, "wrong number of arguments for '{}' command", command)
            }
            Self::InvalidCommandName => write!(f, "invalid command name"),
            Self::EmptyCommand => write!(f, "empty command"),
            Self::InvalidArgumentType => write!(f, "invalid argument type"),
            Self::NotImplemented => write!(f, "command not implemented"),
            Self::UnknownCommand(cmd) => write!(f, "unknown command '{}'", cmd),
            Self::StorageError(e) => write!(f, "storage error: {}", e),
        }
    }
}

impl std::error::Error for CommandError {}

impl Command {
    // 从 RespValue 解析命令
    pub fn from_resp(resp: RespValue) -> Result<Command, Error> {
        match resp {
            RespValue::Array(Some(array)) => {
                if array.is_empty() {
                    return Err(anyhow!(CommandError::EmptyCommand));
                }

                // 获取命令名称
                let command_name = match &array[0] {
                    RespValue::BulkString(Some(s)) | RespValue::SimpleString(s) => s.to_uppercase(),
                    _ => return Err(anyhow!(CommandError::InvalidCommandName)),
                };

                // 解析具体命令
                match command_name.as_str() {
                    "GET" => {
                        if array.len() != 2 {
                            return Err(anyhow!(CommandError::WrongNumberOfArguments {
                                command: "get".to_string()
                            }));
                        }
                        let key = Self::extract_string(&array[1])?;
                        Ok(Command::Get { key })
                    }

                    "SET" => {
                        if array.len() != 3 {
                            return Err(anyhow!(CommandError::WrongNumberOfArguments {
                                command: "set".to_string()
                            }));
                        }
                        let key = Self::extract_string(&array[1])?;
                        let value = Self::extract_string(&array[2])?;
                        Ok(Command::Set { key, value })
                    }

                    "DEL" => {
                        if array.len() < 2 {
                            return Err(anyhow!(CommandError::WrongNumberOfArguments {
                                command: "del".to_string()
                            }));
                        }
                        let keys = array[1..]
                            .iter()
                            .map(Self::extract_string)
                            .collect::<Result<Vec<_>, _>>()?;
                        Ok(Command::Del { keys })
                    }

                    "LPUSH" => {
                        if array.len() < 3 {
                            return Err(anyhow!(CommandError::WrongNumberOfArguments {
                                command: "lpush".to_string()
                            }));
                        }
                        let key = Self::extract_string(&array[1])?;
                        let values = array[2..]
                            .iter()
                            .map(Self::extract_string)
                            .collect::<Result<Vec<_>, _>>()?;
                        Ok(Command::LPush { key, values })
                    }

                    "PING" => Ok(Command::Ping),

                    "INFO" => Ok(Command::Info),
                    "COMMAND" => Ok(Command::Command),

                    _ => Ok(Command::Unknown {
                        command: command_name,
                    }),
                }
            }
            _ => Err(anyhow!(CommandError::InvalidCommandName)),
        }
    }

    // 辅助函数：从 RespValue 提取字符串
    fn extract_string(value: &RespValue) -> Result<String, Error> {
        match value {
            RespValue::BulkString(Some(s)) | RespValue::SimpleString(s) => Ok(s.to_string()),
            _ => Err(anyhow!(CommandError::InvalidArgumentType)),
        }
    }

    pub async fn exec<S>(
        self,
        db: Arc<DB<S, String, RespValue<'static>>>,
    ) -> Result<RespValue<'static>, Error>
    where
        S: Storage<String, RespValue<'static>> + 'static,
    {
        match self {
            Command::Get { key } => {
                match db.get(&key).map_err(|e| CommandError::StorageError(e))? {
                    Some(value) => Ok(value),
                    None => Ok(RespValue::Null),
                }
            }
            Command::Set { key, value } => {
                db.set(
                    key.clone(),
                    RespValue::BulkString(Some(Cow::Owned(value.clone()))),
                )
                .map_err(|e| CommandError::StorageError(e))?;
                Ok(RespValue::SimpleString(Cow::Borrowed("OK")))
            }
            Command::Ping => Ok(RespValue::SimpleString(Cow::Borrowed("PONG"))),
            Command::Unknown { command } => Err(anyhow!(CommandError::UnknownCommand(command))),
            Command::Info => Ok(RespValue::BulkString(Some(Cow::Owned(format!(
                "redis_version:1.0.0\r\nredis_mode:standalone"
            ))))),
            Command::Command => Ok(RespValue::SimpleString(Cow::Borrowed("OK"))),
            _ => Err(anyhow!(CommandError::NotImplemented)),
        }
    }
}

impl CommandError {
    pub fn as_error_msg(&self) -> &'static str {
        match self {
            Self::WrongNumberOfArguments { .. } => "-ERR wrong number of arguments",
            Self::InvalidCommandName => "-ERR invalid command name",
            Self::EmptyCommand => "-ERR empty command",
            Self::InvalidArgumentType => "-ERR invalid argument type",
            Self::NotImplemented => "-ERR command not implemented",
            Self::UnknownCommand(_) => "-ERR unknown command",
            Self::StorageError(_) => "-ERR storage error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_get_command() {
        let resp = RespValue::Array(Some(vec![
            RespValue::BulkString(Some(Cow::Owned("GET".to_string()))),
            RespValue::BulkString(Some(Cow::Owned("mykey".to_string()))),
        ]));

        match Command::from_resp(resp) {
            Ok(Command::Get { key }) => assert_eq!(key, "mykey"),
            _ => panic!("Failed to parse GET command"),
        }
    }

    #[test]
    fn test_parse_set_command() {
        let resp = RespValue::Array(Some(vec![
            RespValue::BulkString(Some(Cow::Owned("SET".to_string()))),
            RespValue::BulkString(Some(Cow::Owned("mykey".to_string()))),
            RespValue::BulkString(Some(Cow::Owned("myvalue".to_string()))),
        ]));

        match Command::from_resp(resp) {
            Ok(Command::Set { key, value }) => {
                assert_eq!(key, "mykey");
                assert_eq!(value, "myvalue");
            }
            _ => panic!("Failed to parse SET command"),
        }
    }

    #[test]
    fn test_invalid_command() {
        let resp = RespValue::SimpleString(Cow::Owned("NOT_AN_ARRAY".to_string()));
        assert!(Command::from_resp(resp).is_err());
    }
}

//EOF
