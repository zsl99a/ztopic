use helium::{framed_msgpack, HandlerSymbol, P2pRt, Service};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    P2pRt::new(
        Service::new()
            .add_handler(HandlerSymbol::new("demo"), |framed_io, p2p_rt| async move {
                let framed_serde = framed_msgpack::<Msg>(framed_io);
            })
            .add_handler(HandlerSymbol::new("demo2"), |framed_io, p2p_rt| async move {
                let framed_serde = framed_msgpack::<Msg>(framed_io);
            }),
    )
    .await?
    .spawn_with_addr("[::]:0".parse()?)
    .await?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Msg {
    pub id: u64,
    pub name: String,
}
