use futures::{SinkExt, StreamExt};
use helium::{framed_msgpack, FramedIO, P2pRt, Service};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = Service::new()
        .add_service("master", |framed_io, p2p_rt| async move {
            let mut framed_serde = framed_msgpack::<Msg>(framed_io);

            println!("master");

            tokio::spawn(async move {
                while let Some(Ok(msg)) = framed_serde.next().await {
                    framed_serde.send(msg).await?;
                }
                anyhow::Result::<()>::Ok(())
            });
        })
        .add_service("quote", |framed_io, p2p_rt| async move {
            let framed_serde = framed_msgpack::<Msg>(framed_io);
        })
        .add_service("node", |framed_io, p2p_rt| async move {
            ChildService::new(framed_io, p2p_rt).spawn().await;
        });

    P2pRt::new(service).await?.spawn_with_addr("0.0.0.0:31234".parse()?).await?;

    let p2p_rt = P2pRt::new(Service::new()).await?.spawn_with_addr("0.0.0.0:31235".parse()?).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let framed_io = p2p_rt.open_stream("127.0.0.1:31234".parse()?, "master").await?;
    let mut framed_serde = framed_msgpack::<Msg>(framed_io);

    framed_serde.send(Msg { id: 1, name: "hello".into() }).await?;

    while let Some(Ok(msg)) = framed_serde.next().await {
        println!("recv: {:?}", msg);
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Msg {
    pub id: u64,
    pub name: String,
}

pub struct ChildService {}

impl ChildService {
    pub fn new(framed_io: FramedIO, p2p_rt: P2pRt) -> Self {
        Self {}
    }

    async fn spawn(&self) {}
}
