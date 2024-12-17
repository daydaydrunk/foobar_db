#![warn(unused_imports)]
use anyhow::Result;
use bytes::{Buf, BytesMut};
use socket2::Socket;
use std::sync::Arc;
use std::time::Duration;
use stream_resp::parser::Parser;
use stream_resp::resp::RespValue;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;
use tracing::error;

const INITIAL_BUFFER_SIZE: usize = 4096;
const MAX_BATCH_SIZE: usize = 1024;

use crate::{
    db::{db::DB, storage::DashMapStorage},
    protocal::command::Command,
};

pub struct ClientConn {
    reader: tokio::io::BufReader<tokio::io::ReadHalf<TcpStream>>,
    writer: BufWriter<tokio::io::WriteHalf<TcpStream>>,
    db: Arc<DB<DashMapStorage<String, RespValue<'static>>, String, RespValue<'static>>>,
    parser: Parser,
    peer_addr: std::net::SocketAddr,
    read_buf: BytesMut,
    write_buf: BytesMut,
}

impl ClientConn {
    pub fn new(
        stream: TcpStream,
        db: Arc<DB<DashMapStorage<String, RespValue<'static>>, String, RespValue<'static>>>,
    ) -> Self {
        // 优化TCP配置
        stream.set_nodelay(true).unwrap();
        let addr = stream.peer_addr().unwrap();
        let (rd, wr) = tokio::io::split(stream);
        let reader = tokio::io::BufReader::with_capacity(INITIAL_BUFFER_SIZE, rd);
        let writer = BufWriter::with_capacity(INITIAL_BUFFER_SIZE, wr);

        Self {
            reader,
            writer,
            db,
            parser: Parser::new(10, 1024),
            peer_addr: addr,
            read_buf: BytesMut::with_capacity(INITIAL_BUFFER_SIZE),
            write_buf: BytesMut::with_capacity(INITIAL_BUFFER_SIZE),
        }
    }

    async fn write_response(&mut self, response: &[u8]) -> Result<()> {
        self.writer.write_all(response).await?;
        self.writer.flush().await?;
        Ok(())
    }

    #[inline(always)]
    pub async fn handle_connection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut batch = Vec::with_capacity(MAX_BATCH_SIZE);

        loop {
            match self.reader.read_buf(&mut self.parser.buffer).await {
                Ok(0) => break,
                Ok(_) => {
                    while let Ok(Some(resp)) = self.parser.try_parse() {
                        if let Ok(cmd) = Command::from_resp(resp) {
                            batch.push(cmd);

                            if batch.len() >= MAX_BATCH_SIZE {
                                self.execute_batch(&mut batch).await?;
                            }
                        }
                    }

                    if !batch.is_empty() {
                        self.execute_batch(&mut batch).await?;
                    }
                }
                Err(e) => {
                    error!("Read error from {}: {}", self.peer_addr, e);
                    return Err(e.into());
                }
            }
        }
        Ok(())
    }

    #[inline(always)]
    async fn execute_batch(
        &mut self,
        batch: &mut Vec<Command>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut futures = Vec::with_capacity(batch.len());

        // 并发执行命令
        for cmd in batch.drain(..) {
            futures.push(cmd.exec(self.db.clone()));
        }

        // 等待所有命令完成
        let results = futures::future::join_all(futures).await;

        // 批量写入响应
        for result in results {
            match result {
                Ok(resp) => {
                    self.write_buf.extend(resp.to_owned().as_bytes());
                }
                Err(e) => {
                    self.write_buf.extend(format!("-ERR {}\r\n", e).as_bytes());
                }
            }
        }

        // 一次性写入所有响应
        self.writer.write_all(&self.write_buf).await?;
        self.writer.flush().await?;
        self.write_buf.clear();

        Ok(())
    }
}

//EOF
