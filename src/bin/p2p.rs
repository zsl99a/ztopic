use anyhow::Result;
use helium::{framed_msgpack, FramedIO, P2pRt, Service};
use rbdc::datetime::DateTime;
use serde::{Deserialize, Serialize};
use tarpc::{
    context::Context,
    server::{BaseChannel, Channel},
};

#[tokio::main]
async fn main() -> Result<()> {
    let service = Service::new()
        .add_service("master", |framed_io, p2p_rt| async move {
            NodeImpl.spawn(framed_io, p2p_rt).await;
        })
        .add_service("relay", |framed_io, p2p_rt| async move {
            let framed_serde = framed_msgpack::<Msg>(framed_io);
        })
        .add_service("node", |framed_io, p2p_rt| async move {
            ChildService::new(framed_io, p2p_rt).spawn().await;
        });
    P2pRt::new(service).await?.spawn_with_addr("0.0.0.0:31234".parse()?).await?;

    let p2p_rt = P2pRt::new(Service::new()).await?.spawn_with_addr("0.0.0.0:31235".parse()?).await?;

    let framed_io = p2p_rt.open_stream("127.0.0.1:31234".parse()?, "master").await?;
    let node_client = NodeClient::spawn(framed_io);

    println!("🚀");

    let p = node_client.ping(Context::current(), DateTime::now()).await?.expect("ping");

    println!("🚀 {:?}", p);

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

#[tarpc::service]
pub trait Node {
    async fn ping(time: DateTime) -> Result<(), ()>;
}

impl NodeClient {
    fn spawn(framed_io: FramedIO) -> Self {
        let framed_serde = tarpc::serde_transport::new(framed_io, tokio_serde::formats::MessagePack::default());
        Self::new(tarpc::client::Config::default(), framed_serde).spawn()
    }
}

#[derive(Clone)]
pub struct NodeImpl;

impl NodeImpl {
    pub async fn spawn(self, framed_io: FramedIO, _p2p_rt: P2pRt) {
        let framed_serde = tarpc::serde_transport::new(framed_io, tokio_serde::formats::MessagePack::default());
        BaseChannel::with_defaults(framed_serde).execute(self.serve()).await
    }
}

#[tarpc::server]
impl Node for NodeImpl {
    async fn ping(self, _: Context, time: DateTime) -> Result<(), ()> {
        println!("ping: {:?}", time);
        Ok(())
    }
}
