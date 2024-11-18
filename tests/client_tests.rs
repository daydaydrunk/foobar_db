use foobar_db::server::server::{Server, ServerConfig};
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

async fn send_command(stream: &mut TcpStream, command: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    stream.write_all(command).await?;

    let mut response = vec![0u8; 1024];
    let n = stream.read(&mut response).await?;
    Ok(response[..n].to_vec())
}

#[tokio::test]
async fn test_set_get_commands() -> Result<(), Box<dyn Error>> {
    // 创建并启动服务器
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 6379,
        max_connections: 10,
    };
    let server = Server::new(config);

    // 在新任务中运行服务器
    let server_handle = tokio::spawn(async move {
        server.run().await.unwrap();
    });

    // 等待服务器启动
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 创建客户端连接
    let mut stream = TcpStream::connect("127.0.0.1:6379").await?;

    // 跳过欢迎消息
    let mut welcome = vec![0u8; 1024];
    stream.read(&mut welcome).await?;

    // 测试 SET 命令
    let set_cmd = b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n";
    let response = send_command(&mut stream, set_cmd).await?;
    assert_eq!(&response, b"+OK\r\n");

    // 测试 GET 命令
    let get_cmd = b"*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n";
    let response = send_command(&mut stream, get_cmd).await?;
    assert_eq!(&response, b"$5\r\nvalue\r\n");

    // 关闭连接和服务器
    drop(stream);
    server_handle.abort();

    Ok(())
}

#[tokio::test]
async fn test_multiple_commands() -> Result<(), Box<dyn Error>> {
    // 创建并启动服务器
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 6380, // 使用不同端口避免冲突
        max_connections: 10,
    };
    let server = Server::new(config);

    // 在新任务中运行服务器
    let server_handle = tokio::spawn(async move {
        server.run().await.unwrap();
    });

    // 等待服务器启动
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 创建客户端连接
    let mut stream = TcpStream::connect("127.0.0.1:6380").await?;

    // 跳过欢迎消息
    let mut welcome = vec![0u8; 1024];
    stream.read(&mut welcome).await?;

    // 测试 PING 命令
    let ping_cmd = b"*1\r\n$4\r\nPING\r\n";
    let response = send_command(&mut stream, ping_cmd).await?;
    assert_eq!(&response, b"+PONG\r\n");

    // 测试 SET 命令
    let set_cmd = b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n";
    let response = send_command(&mut stream, set_cmd).await?;
    assert_eq!(&response, b"+OK\r\n");

    // 测试 GET 命令
    let get_cmd = b"*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n";
    let response = send_command(&mut stream, get_cmd).await?;
    assert_eq!(&response, b"$5\r\nvalue\r\n");

    // 测试 GET 不存在的键
    let get_missing_cmd = b"*2\r\n$3\r\nGET\r\n$7\r\nmissing\r\n";
    let response = send_command(&mut stream, get_missing_cmd).await?;
    assert_eq!(&response, b"$-1\r\n");

    // 测试 INFO 命令
    let info_cmd = b"*1\r\n$4\r\nINFO\r\n";
    let response = send_command(&mut stream, info_cmd).await?;
    assert!(response.starts_with(b"$"));
    assert!(response
        .windows(13)
        .position(|w| w == b"redis_version")
        .is_some());
    assert!(response
        .windows(10)
        .position(|w| w == b"redis_mode")
        .is_some());

    // 测试 COMMAND 命令
    let command_cmd = b"*1\r\n$7\r\nCOMMAND\r\n";
    let response = send_command(&mut stream, command_cmd).await?;
    assert_eq!(&response, b"+OK\r\n");

    // 测试未知命令
    let unknown_cmd = b"*1\r\n$7\r\nUNKNOWN\r\n";
    let response = send_command(&mut stream, unknown_cmd).await?;
    assert!(response.starts_with(b"-ERR"));

    // 关闭连接和服务器
    drop(stream);
    server_handle.abort();

    Ok(())
}
