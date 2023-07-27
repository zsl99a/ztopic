use std::{fmt::Debug, net::SocketAddr, sync::Arc};

use anyhow::Result;
use futures::{StreamExt, TryStreamExt};
use parking_lot::Mutex;
use s2n_quic::{connection::Handle, provider::event::default::Subscriber, Client, Connection};
use serde::{de::DeserializeOwned, Serialize};
use tokio_serde::formats::MessagePack;
use tokio_util::codec::LengthDelimitedCodec;

use crate::MtlsProvider;

pub static CA_CERT_PEM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/certs/ca.crt");
pub static MY_CERT_PEM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server.crt");
pub static MY_KEY_PEM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server.key");

///
/// 1. 连接 master 节点, 获取到 master 节点的 connection
/// 2. 通过 connection 创建 node 实例
/// 3. 通过 node 实例注册当前节点到 master 节点
/// 4. master 节点返回 peer 和 master 节点的 services, 并保存到 node 实例
///

#[derive(Clone)]
pub struct P2pRt {
    client: Client,
    node: Arc<Mutex<Node>>,
}

impl P2pRt {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            client: create_client("[::1]:0".parse()?).await?,
            node: Arc::new(Mutex::new(Node::new())),
        })
    }
}

impl P2pRt {
    pub async fn spawn_with_addr<Msg>(&self, addr: SocketAddr) -> Result<Self>
    where
        Msg: DeserializeOwned + Serialize + Debug + Send + Sync + 'static,
    {
        tokio::spawn({
            let this = self.clone();

            let mut server = create_server(addr).await?;
            println!("server addr: {}", server.local_addr()?);

            async move {
                while let Some(conn) = server.accept().await {
                    tokio::spawn(this.clone().serve::<Msg>(conn));
                }
            }
        });

        Ok(self.clone())
    }

    async fn serve<Msg>(self, conn: Connection)
    where
        Msg: DeserializeOwned + Serialize + Debug + Send + Sync + 'static,
    {
        let (handle, mut acceptor) = conn.split();

        while let Ok(Some(stream)) = acceptor.accept_bidirectional_stream().await {
            let framed_io = LengthDelimitedCodec::builder().max_frame_length(1024 * 1024 * 4).new_framed(stream);
            let mut serde_io = Box::pin(tokio_serde::Framed::<_, Msg, Msg, _>::new(framed_io, MessagePack::<Msg, Msg>::default()));

            // if let Some(Ok(msg)) = serde_io.next().await {
            //     println!("msg = {:?}", msg);
            // }
        }
    }
}

pub struct Node {
    peers: Vec<Peer>,
}

impl Node {
    pub fn new() -> Self {
        Self { peers: Vec::new() }
    }
}

pub struct Peer {
    openner: Handle,
}

impl Peer {
    pub fn new(openner: Handle) -> Self {
        Self { openner }
    }
}

// =====

async fn create_client(addr: SocketAddr) -> Result<s2n_quic::Client> {
    let client = s2n_quic::Client::builder()
        .with_event(Subscriber::default())?
        .with_tls(MtlsProvider::new(CA_CERT_PEM, MY_CERT_PEM, MY_KEY_PEM).await?)?
        .with_io(addr)?
        .start()
        .map_err(|e| anyhow::anyhow!("failed to create client: {:?}", e))?;

    Ok(client)
}

async fn create_server(addr: SocketAddr) -> Result<s2n_quic::Server> {
    let server = s2n_quic::Server::builder()
        .with_event(Subscriber::default())?
        .with_tls(MtlsProvider::new(CA_CERT_PEM, MY_CERT_PEM, MY_KEY_PEM).await?)?
        .with_io(addr)?
        .start()
        .map_err(|e| anyhow::anyhow!("failed to create server: {:?}", e))?;

    Ok(server)
}
