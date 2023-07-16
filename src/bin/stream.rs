use std::time::Instant;

use futures::{StreamExt, SinkExt};
use helium::SharedStream;
use tokio::{time::Duration, task::yield_now};

#[tokio::main]
async fn main() {
    let all = 10000;

    let (mut tx, rx) = futures::channel::mpsc::channel(32);

    tokio::spawn(async move {
        for i in 0..all {
            tx.send(i).await.unwrap();
            yield_now().await;
        }
    });

    // let rx = futures::stream::iter(0..all).map(|i| i);

    let s = SharedStream::new(rx);

    tokio::spawn({
        let mut s = s.clone();

        async move {
            let ins = Instant::now();

            while let Some(i) = s.next().await {
                println!("a = {:?}", i);

                if i == all - 1 {
                    println!("a = {}, time = {:?}", i, ins.elapsed());
                }
            }

            println!("a = {}, time = {:?}", all - 1, ins.elapsed());
        }
    });

    tokio::spawn({
        let mut s = s.clone();

        async move {
            let ins = Instant::now();

            while let Some(i) = s.next().await {
                println!("b = {:?}", i);

                if i == all - 1 {
                    println!("b = {}, time = {:?}", i, ins.elapsed());
                }
            }

            println!("b = {}, time = {:?}", all - 1, ins.elapsed());
        }
    });

    tokio::spawn({
        let mut s = s.clone();

        async move {
            let ins = Instant::now();

            while let Some(i) = s.next().await {
                println!("c = {:?}", i);

                if i == all - 1 {
                    println!("c = {}, time = {:?}", i, ins.elapsed());
                }
            }

            println!("c = {}, time = {:?}", all - 1, ins.elapsed());
        }
    });

    tokio::spawn({
        let mut s = s.clone();

        async move {
            let ins = Instant::now();

            while let Some(i) = s.next().await {
                println!("d = {:?}", i);

                if i == all - 1 {
                    println!("d = {}, time = {:?}", i, ins.elapsed());
                }
            }

            println!("d = {}, time = {:?}", all - 1, ins.elapsed());
        }
    });

    tokio::spawn({
        let mut s = s.clone();

        async move {
            let ins = Instant::now();

            while let Some(i) = s.next().await {
                println!("e = {:?}", i);

                if i == all - 1 {
                    println!("e = {}, time = {:?}", i, ins.elapsed());
                }
            }

            println!("e = {}, time = {:?}", all - 1, ins.elapsed());
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;
}
