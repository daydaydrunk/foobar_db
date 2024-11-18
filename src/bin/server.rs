use clap::Parser;
use foobar_db::server::server::{Server, ServerConfig};
use num_cpus;
use std::fs;
use tokio::runtime::Builder;
use tokio::signal;
use tracing::info;
use vergen::{BuildBuilder, CargoBuilder, Emitter, RustcBuilder, SysinfoBuilder};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Config {
    #[arg(short = 'H', long = "host", default_value = "127.0.0.1")]
    host: String,

    #[arg(short = 'P', long = "port", default_value = "6379")]
    port: u16,

    #[arg(short = 'M', long = "max-connections", default_value = "1000")]
    max_connections: usize,

    #[arg(short = 'b', long = "build info")]
    build_info: bool,
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
    tracing_subscriber::fmt::init();

    let config = Config::parse();

    if config.build_info {
        print_build_info();
        return;
    }

    let server_config = ServerConfig {
        host: config.host,
        port: config.port,
        max_connections: config.max_connections,
    };

    print_banner();

    let server = Server::new(server_config);

    info!("Starting server...");

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

fn print_build_info() {
    let mut e = Emitter::default();
    if let Ok(build) = BuildBuilder::all_build() {
        _ = e.add_instructions(&build);
    }
    if let Ok(build) = CargoBuilder::all_cargo() {
        _ = e.add_instructions(&build);
    }
    if let Ok(build) = RustcBuilder::all_rustc() {
        _ = e.add_instructions(&build);
    }
    if let Ok(build) = SysinfoBuilder::all_sysinfo() {
        _ = e.add_instructions(&build);
    }
    _ = e.emit();
}

//EOF
