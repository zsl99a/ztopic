use std::time::Duration;

use anyhow::Result;
use tokio::task::JoinSet;

#[tokio::main]
async fn main() -> Result<()> {
    let mut join_set = JoinSet::new();

    join_set.spawn(async {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            println!("hello")
        }
    });

    let join_set = Box::leak(Box::new(join_set));

    tokio::time::sleep(Duration::from_secs(3)).await;

    unsafe { std::ptr::drop_in_place(join_set) };

    tokio::time::sleep(Duration::from_secs(10)).await;

    Ok(())
}
