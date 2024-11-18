# FoobarDB

FoobarDB is an in-memory database implemented in Rust that supports the RESP V3 protocol. This project is for learning and research purposes. I aim to familiarize myself with Rust coding by implementing an in-memory database.

## Features

- Supports RESP protocol
- Supports common Redis commands
- High performance and low latency
- Supports asynchronous operations

### Prerequisites

- Rust 1.82.0
- Cargo package manager

### Clone the repository

\`\`\`sh
git clone https://github.com/yourusername/foobardb.git
cd foobardb
\`\`\`

### Build the project

\`\`\`sh
cargo build --release
\`\`\`

## Usage

### Start the server

\`\`\`sh
cargo run --release --bin foobar_db
\`\`\`

### Client example

You can use any client that supports the RESP protocol to interact with FoobarDB. Here is a simple example using \`redis-cli\` to connect to FoobarDB:

\`\`\`sh
redis-cli -h 127.0.0.1 -p 6379
\`\`\`

In \`redis-cli\`, you can execute the following commands:

\`\`\`sh
SET mykey "Hello, World!"
GET mykey
DEL mykey
\`\`\`


## Contributing

We welcome contributions! Please follow these steps:

1. Fork this repository
2. Create your feature branch (\`git checkout -b feature/fooBar\`)
3. Commit your changes (\`git commit -am 'Add some fooBar'\`)
4. Push to the branch (\`git push origin feature/fooBar\`)
5. Create a new Pull Request

## License

FoobarDB is licensed under the MIT License. For more information, please see the [LICENSE](LICENSE) file.

## Authors

- [](https://github.com/daydaydrunk/foobar_db)

## References

- [Redis](https://redis.io/)
- [Tokio](https://tokio.rs/)
- [DashMap](https://github.com/xacrimon/dashmap)