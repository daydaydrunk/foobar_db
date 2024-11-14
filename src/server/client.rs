use anyhow::Result;
use bytes::BytesMut;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::{
    db::{db::DB, storage::DashMapStorage},
    protocal::{command::Command, parser::Parser, resp::RespValue},
};

pub struct ClientConn {
    reader: tokio::io::ReadHalf<TcpStream>,
    writer: tokio::io::WriteHalf<TcpStream>,
    db: Arc<DB<DashMapStorage<String, RespValue<'static>>, String, RespValue<'static>>>,
    pub parser: Parser,
    peer_addr: std::net::SocketAddr,
}

impl ClientConn {
    pub fn new(
        stream: TcpStream,
        db: Arc<DB<DashMapStorage<String, RespValue<'static>>, String, RespValue<'static>>>,
    ) -> Self {
        let peer_addr = stream.peer_addr().expect("Failed to get peer address");
        stream.set_nodelay(true).expect("Failed to set TCP_NODELAY");
        let (reader, writer) = tokio::io::split(stream);

        Self {
            reader,
            writer,
            db,
            parser: Parser::new(10, 1000),
            peer_addr,
        }
    }

    async fn write_response(&mut self, response: &[u8]) -> Result<()> {
        self.writer.write_all(response).await?;
        self.writer.flush().await?;
        Ok(())
    }

    pub async fn handle_connection(&mut self) -> Result<()> {
        let socket_addr = self.peer_addr;

        loop {
            let n = match self.reader.read_buf(&mut self.parser.buffer).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => return Err(e.into()),
            };

            while let Ok(Some(resp)) = self.parser.try_parse() {
                match Command::from_resp(resp) {
                    Ok(cmd) => {
                        let response = match cmd.exec(self.db.clone()).await {
                            Ok(resp) => resp.to_owned().as_bytes(),
                            Err(e) => format!("-ERR {}\r\n", e).as_bytes().to_vec(),
                        };
                        self.write_response(&response).await?;
                    }
                    Err(e) => {
                        self.write_response(format!("-ERR invalid command {}\r\n", e).as_bytes())
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }
}
