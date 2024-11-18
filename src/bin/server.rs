use clap::Parser;
use foobar_db::server::server::{Server, ServerConfig}; // 替换 your_crate_name 为你的 crate 名
use num_cpus;
use std::fs;
use tokio::runtime::Builder;
use tokio::signal;
use tracing::info;

/// 命令行参数配置
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Config {
    /// 服务器主机地址
    #[arg(short = 'H', long = "host", default_value = "127.0.0.1")]
    host: String,

    /// 服务器端口
    #[arg(short = 'P', long = "port", default_value = "6379")]
    port: u16,

    /// 最大连接数
    #[arg(short = 'M', long = "max-connections", default_value = "1000")]
    max_connections: usize,
}

async fn run_server(mut server: Server) {
    // Set up signal handler
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    // Run server until Ctrl+C is received
    tokio::select! {
        _ = server.run() => {},
        _ = ctrl_c => {
            server.close().await;
        }
    }
}

fn main() {
    print_banner();

    // Initialize logger
    tracing_subscriber::fmt::init();

    // 解析命令行参数
    let config = Config::parse();

    // 创建服务器配置
    let server_config = ServerConfig {
        host: config.host,
        port: config.port,
        max_connections: config.max_connections,
    };

    // 初始化服务器
    let server = Server::new(server_config);

    // 启动服务器
    info!("Starting server...");
    // 创建多线程运行时
    let runtime: tokio::runtime::Runtime = Builder::new_multi_thread()
        .worker_threads(num_cpus::get())
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    runtime.block_on(async {
        run_server(server).await;
    });
}

fn print_banner() {
    if let Ok(banner) = fs::read_to_string("assets/banner.txt") {
        println!("{}", banner);
    }
}

//EOF
