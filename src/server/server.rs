use crate::db::db::DB;
use crate::db::storage::DashMapStorage;
use crate::protocal::resp::RespValue;
use crate::server::client::ClientConn;
use std::error::Error;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 6379,
            max_connections: 1000,
        }
    }
}

pub struct Server {
    config: ServerConfig,
    db: Arc<DB<DashMapStorage<String, RespValue<'static>>, String, RespValue<'static>>>,
    listener: Option<TcpListener>,
    connections: Option<
        Vec<(
            TcpStream,
            Arc<DB<DashMapStorage<String, RespValue<'static>>, String, RespValue<'static>>>,
        )>,
    >,
    handle: Option<tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>>,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        let storage = DashMapStorage::new();
        let db = DB::new(storage);
        Self {
            config,
            db: Arc::new(db),
            listener: None,
            connections: None,
            handle: None,
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await?;
        println!("Server listening on {}", addr);

        loop {
            let (socket, _) = listener.accept().await?;
            let db = self.db.clone();

            tokio::spawn(async move {
                if let Err(e) = ClientConn::new(socket, db).handle_connection().await {
                    eprintln!("Error handling connection: {}", e);
                }
            });
        }
    }

    pub async fn close(&mut self) {
        // Close listener
        if let Some(listener) = self.listener.take() {
            drop(listener);
        }

        // Close all active connections
        if let Some(connections) = self.connections.take() {
            for (_, conn) in connections {
                drop(conn);
            }
        }

        // Cancel any running tasks
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

//EOF
