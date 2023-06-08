use std::net::TcpListener;

use z2p::run;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    run(TcpListener::bind("127.0.0.1:8000")?)?.await
}
