use anyhow::Result;
use std::net::{IpAddr, Ipv4Addr};

pub async fn has_internet_connection() -> bool {
    ping_cloudflare().await.is_ok()
}

async fn ping_cloudflare() -> Result<()> {
    // Really crude, but should be OK?
    // Some networks seem to have issues pinging 1.1.1.1 (see https://community.cloudflare.com/t/cant-ping-or-access-1-1-1-1/346202),
    // so we ping their alternative DNS address instead.
    let ip = IpAddr::V4(Ipv4Addr::new(1, 0, 0, 1));
    let payload = [0; 8];

    surge_ping::ping(ip, &payload).await?;

    Ok(())
}
