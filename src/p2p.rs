use std::{collections::HashMap, fmt::Debug, net::SocketAddr, pin::Pin, sync::Arc};

use anyhow::Result;
use futures::{Future, SinkExt, StreamExt};
use parking_lot::Mutex;
use s2n_quic::{client::Connect, connection::Handle, provider::event::default::Subscriber, stream::BidirectionalStream, Client, Connection};
use serde::{Deserialize, Serialize};
use tokio_serde::formats;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

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
    service: Arc<Service>,
}

impl P2pRt {
    pub async fn new(service: Service) -> Result<Self> {
        Ok(Self {
            client: create_client("[::1]:0".parse()?).await?,
            node: Arc::new(Mutex::new(Node::new())),
            service: Arc::new(service),
        })
    }

    pub async fn open_stream(&self, addr: SocketAddr, symbol: impl Into<HandlerSymbol>) -> Result<Framed<BidirectionalStream, LengthDelimitedCodec>> {
        if self.node.lock().peers.iter().find(|peer| peer.openner.remote_addr() == Ok(addr)).is_none() {
            let mut conn = self.client.connect(Connect::new(addr).with_server_name("localhost")).await?;
            conn.keep_alive(true)?;
            tokio::spawn(self.clone().serve(conn));
        }

        let mut openner = self
            .node
            .lock()
            .peers
            .iter()
            .find(|peer| peer.openner.remote_addr() == Ok(addr))
            .ok_or(anyhow::anyhow!("no peer"))?
            .openner
            .clone();

        let stream = openner.open_bidirectional_stream().await?;
        let mut framed_io = LengthDelimitedCodec::builder().max_frame_length(1024 * 1024 * 4).new_framed(stream);

        let negotiate = Negotiate { handler: symbol.into() };
        framed_io.send(rmp_serde::to_vec(&negotiate)?.into()).await?;

        Ok(framed_io)
    }
}

impl P2pRt {
    pub async fn spawn_with_addr(self, addr: SocketAddr) -> Result<Self> {
        let this = self.clone();

        let mut server = create_server(addr).await?;
        println!("server addr: {}", server.local_addr()?);

        tokio::spawn(async move {
            while let Some(conn) = server.accept().await {
                tokio::spawn(this.clone().serve(conn));
            }
        });

        Ok(self.clone())
    }

    async fn serve(self, conn: Connection) {
        let (handle, mut acceptor) = conn.split();

        self.node.lock().peers.push(Peer::new(handle.clone()));

        while let Ok(Some(stream)) = acceptor.accept_bidirectional_stream().await {
            let this = self.clone();

            tokio::spawn(async move {
                let mut framed_io = LengthDelimitedCodec::builder().max_frame_length(1024 * 1024 * 4).new_framed(stream);

                let bytes = framed_io.next().await.ok_or(anyhow::anyhow!("no bytes"))??;
                let negotiate = rmp_serde::from_slice::<Negotiate>(&bytes).map_err(|e| anyhow::anyhow!("rmp_serde::from_slice: {}", e))?;

                let handler = this.service.svcs.get(&negotiate.handler).ok_or(anyhow::anyhow!("no handler"))?;

                handler(framed_io, this.clone()).await;

                Result::<()>::Ok(())
            });
        }

        self.node.lock().peers.retain(|peer| peer.openner.remote_addr() != handle.remote_addr());
    }
}

pub fn framed_msgpack<Msg>(
    framed_io: Framed<BidirectionalStream, LengthDelimitedCodec>,
) -> tokio_serde::Framed<Framed<BidirectionalStream, LengthDelimitedCodec>, Msg, Msg, formats::MessagePack<Msg, Msg>> {
    tokio_serde::Framed::new(framed_io, formats::MessagePack::default())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Negotiate {
    handler: HandlerSymbol,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct HandlerSymbol(String);

impl HandlerSymbol {
    pub fn new<I: Into<String>>(name: I) -> Self {
        Self(name.into())
    }
}

impl From<&str> for HandlerSymbol {
    fn from(name: &str) -> Self {
        Self::new(name)
    }
}

impl From<String> for HandlerSymbol {
    fn from(name: String) -> Self {
        Self::new(name)
    }
}

// =====

#[derive(Debug, Clone)]
pub struct Node {
    peers: Vec<Peer>,
}

impl Node {
    pub fn new() -> Self {
        Self { peers: Vec::new() }
    }
}

#[derive(Debug, Clone)]
pub struct Peer {
    openner: Handle,
}

impl Peer {
    pub fn new(openner: Handle) -> Self {
        Self { openner }
    }
}

pub struct Service {
    svcs:
        HashMap<HandlerSymbol, Box<dyn Fn(Framed<BidirectionalStream, LengthDelimitedCodec>, P2pRt) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>>,
}

impl Service {
    pub fn new() -> Self {
        Self { svcs: HashMap::new() }
    }

    pub fn add_handler<S, H, F>(mut self, name: S, handler: H) -> Self
    where
        S: Into<HandlerSymbol>,
        H: Fn(Framed<BidirectionalStream, LengthDelimitedCodec>, P2pRt) -> F + Send + Sync + 'static,
        F: Future<Output = ()> + Send + 'static,
    {
        self.svcs
            .insert(name.into(), Box::new(move |framed_io, p2p_rt| Box::pin(handler(framed_io, p2p_rt))));
        self
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
