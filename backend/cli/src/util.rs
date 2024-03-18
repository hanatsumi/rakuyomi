use anyhow::Result;
use std::net::{IpAddr, Ipv4Addr};

pub async fn has_internet_connection() -> bool {
    ping_cloudflare().await.is_ok()
}

async fn ping_cloudflare() -> Result<()> {
    // Really crude, but should be OK?
    let ip = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));
    let payload = [0; 8];

    surge_ping::ping(ip, &payload).await?;

    Ok(())
}
