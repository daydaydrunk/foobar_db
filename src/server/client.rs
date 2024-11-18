use anyhow::Result;
use bytes::BytesMut;
use socket2::{Domain, Socket, Type};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, error, info, trace, warn};

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

        // Convert to socket2::Socket to set keepalive
        let std_stream = stream
            .into_std()
            .expect("Failed to convert to std::net::TcpStream");
        let socket = Socket::from(std_stream);

        // Configure keepalive
        socket.set_keepalive(true).expect("Failed to set keepalive");
        socket
            .set_tcp_keepalive(
                &socket2::TcpKeepalive::new()
                    .with_time(Duration::from_secs(60)) // Keepalive interval
                    .with_interval(Duration::from_secs(10)) // Probe interval
                    .with_retries(3), // Max retries
            )
            .expect("Failed to set keepalive params");

        // Convert back to tokio::TcpStream
        let stream =
            TcpStream::from_std(socket.into()).expect("Failed to convert back to tokio::TcpStream");

        stream.set_nodelay(true).expect("Failed to set TCP_NODELAY");
        let (reader, writer) = tokio::io::split(stream);

        Self {
            reader,
            writer,
            db,
            parser: Parser::new(16, 1024),
            peer_addr,
        }
    }

    async fn write_response(&mut self, response: &[u8]) -> Result<()> {
        self.writer.write_all(response).await?;
        self.writer.flush().await?;
        Ok(())
    }

    pub async fn handle_connection(&mut self) -> Result<()> {
        loop {
            let n = match self.reader.read_buf(&mut self.parser.buffer).await {
                Ok(0) => {
                    debug!("Connection closed by peer: {}", self.peer_addr);
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    error!("Error reading from connection {}: {:?}", self.peer_addr, e);
                    return Err(e.into());
                }
            };
            self.parser.clear_buffer();
            // while let Ok(Some(resp)) = self.parser.try_parse() {
            //     match Command::from_resp(resp) {
            //         Ok(cmd) => {
            //             let response = match cmd.exec(self.db.clone()).await {
            //                 Ok(resp) => resp.to_owned().as_bytes(),
            //                 Err(e) => format!("-ERR {}\r\n", e).as_bytes().to_vec(),
            //             };
            //             self.write_response(&response).await?;
            //         }
            //         Err(e) => {
            //             self.write_response(format!("-ERR invalid command {}\r\n", e).as_bytes())
            //                 .await?;
            //         }
            //     }
            // }
            self.write_response(b"+ok\r\n").await?;
        }

        Ok(())
    }
}
