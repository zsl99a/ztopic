use helium::P2pRt;
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let p2p = P2pRt::new().await?.spawn_with_addr::<Msg>("[::]:0".parse()?).await?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Msg {}
