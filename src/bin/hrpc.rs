use std::error::Error;

use helium::Hrpc;
use tarpc::context::Context;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    Hrpc::new()
        .with_handler(|s| async move {
            println!("s = {:?}", s);
        })
        .serve("[::1]:1234".parse()?)
        .await?;

    Ok(())
}

#[tarpc::service]
pub trait Node {
    async fn get(key: String) -> Result<String, String>;
}

#[derive(Clone)]
pub struct NodeServer;

impl NodeServer {
    pub fn new() -> Self {
        Self
    }
}

#[tarpc::server]
impl Node for NodeServer {
    async fn get(self, _: Context, key: String) -> Result<String, String> {
        Ok(key)
    }
}
