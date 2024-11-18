use crate::db::db::DB;
use crate::db::storage::DashMapStorage;
use crate::protocal::resp::RespValue;
use crate::server::client::ClientConn;
use std::error::Error;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tracing::{debug, error, info, trace, warn};

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
    handle: Option<tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>>,
    shutdown_tx: Option<broadcast::Sender<()>>,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        let storage = DashMapStorage::new();
        let db = DB::new(storage);
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            config,
            db: Arc::new(db),
            shutdown_tx: Some(shutdown_tx),
            listener: None,
            handle: None,
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await?;
        info!("Server listening on {}", addr);

        let shutdown_tx = self.shutdown_tx.clone().unwrap();

        loop {
            let (socket, addr) = listener.accept().await?;
            let db = self.db.clone();
            let mut shutdown_rx = shutdown_tx.subscribe();
            debug!("Accepted connections from {:?}", addr);
            tokio::spawn(async move {
                let mut client_conn = ClientConn::new(socket, db);
                tokio::select! {
                    res = client_conn.handle_connection() => {
                        if let Err(e) = res {
                            error!("Error handling connection: {}", e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        debug!("Received shutdown signal, closing connection from {:?}", addr);
                    }
                }
            });
        }
    }

    pub async fn close(&mut self) {
        // Close listener
        if let Some(listener) = self.listener.take() {
            drop(listener);
        }

        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        info!("Server is shutting down");

        // Cancel any running tasks
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        info!("Exit")
    }
}

//EOF
