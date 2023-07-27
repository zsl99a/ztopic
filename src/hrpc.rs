use std::{error::Error, net::SocketAddr, sync::Arc};

use futures::Future;
use parking_lot::Mutex;
use s2n_quic::{client::Connect, connection::Handle, provider::event::disabled::Subscriber, stream::BidirectionalStream, Client, Connection, Server};
use serde::{Deserialize, Serialize};

use crate::MtlsProvider;

pub static CA_CERT_PEM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/certs/ca.crt");
pub static MY_CERT_PEM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server.crt");
pub static MY_KEY_PEM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server.key");

pub struct Hrpc<F, Fut>
where
    F: Fn(BidirectionalStream) -> Fut + Send + Sync + Clone + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    handler: Option<F>,
    context: Context,
}

impl<F, Fut> Hrpc<F, Fut>
where
    F: Fn(BidirectionalStream) -> Fut + Send + Sync + Clone + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    pub fn new() -> Self {
        Self {
            handler: None,
            context: Context::new(),
        }
    }

    pub fn with_handler(mut self, handler: F) -> Self {
        self.handler = Some(handler);
        self
    }

    pub async fn part(self, addr: SocketAddr) -> Result<Self, Box<dyn Error>> {
        let client = self.create_client(addr).await?;
        let mut conn = client.connect(Connect::new(addr).with_server_name("localhost")).await?;
        conn.keep_alive(true)?;
        self.create_peer(conn);
        Ok(self)
    }

    pub async fn serve(self, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
        let mut server = self.create_server(addr).await?;
        while let Some(conn) = server.accept().await {
            self.create_peer(conn);
        }
        Ok(())
    }

    fn create_peer(&self, conn: Connection) {
        let handler = self.handler.clone().expect("handler not set");

        let (handle, mut acceptor) = conn.split();
        let peer = Peer::new(handle);

        tokio::spawn(async move {
            while let Ok(Some(stream)) = acceptor.accept_bidirectional_stream().await {
                tokio::spawn(handler(stream));
            }
        });
    }

    async fn create_client(&self, addr: SocketAddr) -> Result<Client, Box<dyn Error>> {
        let client = Client::builder()
            .with_event(Subscriber)?
            .with_tls(MtlsProvider::new(CA_CERT_PEM, MY_CERT_PEM, MY_KEY_PEM).await?)?
            .with_io(addr)?
            .start()?;

        Ok(client)
    }

    async fn create_server(&self, addr: SocketAddr) -> Result<Server, Box<dyn Error>> {
        let server = Server::builder()
            .with_event(Subscriber)?
            .with_tls(MtlsProvider::new(CA_CERT_PEM, MY_CERT_PEM, MY_KEY_PEM).await?)?
            .with_io(addr)?
            .start()?;

        Ok(server)
    }
}

#[derive(Debug, Clone)]
pub struct Context {
    addr: SocketAddr,
    peers: Arc<Mutex<Vec<Peer>>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            addr: "[::]:0".parse().unwrap(),
            peers: Arc::new(Mutex::new(vec![])),
        }
    }

    pub async fn online(&self) {}
}

#[derive(Debug, Clone)]
pub struct Peer {
    openner: Handle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub addr: SocketAddr,
}

impl Peer {
    pub fn new(openner: Handle) -> Self {
        Self { openner }
    }

    fn online(&self) {}
}
