use std::net::SocketAddr;

use anyhow::Result;
use tokio::net::TcpStream;

pub async fn has_internet_connection() -> bool {
    try_connecting_to_cloudflare().await.is_ok()
}

async fn try_connecting_to_cloudflare() -> Result<()> {
    let addrs = [
        SocketAddr::from(([1, 0, 0, 1], 80)),
        SocketAddr::from(([1, 1, 1, 1], 80)),
    ];

    TcpStream::connect(&addrs[..]).await?;

    Ok(())
}
