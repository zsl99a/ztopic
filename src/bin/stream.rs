use std::time::Instant;

use futures::StreamExt;
use helium::SharedStream;
use tokio::time::Duration;

#[tokio::main]
async fn main() {
    let all = 10000;

    // let (mut tx, rx) = futures::channel::mpsc::channel(32);

    // tokio::spawn(async move {
    //     for i in 0..all {
    //         tx.send(i).await.unwrap();
    //         yield_now().await;
    //     }
    // });

    let rx = futures::stream::iter(0..all).map(|i| i);

    let s = SharedStream::new(rx);

    tokio::spawn({
        let mut s = s.clone();

        async move {
            let ins = Instant::now();

            while let Some(i) = s.next().await {
                // println!("a = {:?}", i);

                if i == all - 1 {
                    println!("i = {}, time = {:?}", i, ins.elapsed());
                }
            }
        }
    });

    tokio::spawn({
        let mut s = s.clone();

        async move {
            let ins = Instant::now();

            while let Some(i) = s.next().await {
                // println!("a = {:?}", i);

                if i == all - 1 {
                    println!("i = {}, time = {:?}", i, ins.elapsed());
                }
            }
        }
    });

    tokio::spawn({
        let mut s = s.clone();

        async move {
            let ins = Instant::now();

            while let Some(i) = s.next().await {
                // println!("a = {:?}", i);

                if i == all - 1 {
                    println!("i = {}, time = {:?}", i, ins.elapsed());
                }
            }
        }
    });

    tokio::spawn({
        let mut s = s.clone();

        async move {
            let ins = Instant::now();

            while let Some(i) = s.next().await {
                // println!("a = {:?}", i);

                if i == all - 1 {
                    println!("i = {}, time = {:?}", i, ins.elapsed());
                }
            }
        }
    });

    tokio::spawn({
        let mut s = s.clone();

        async move {
            let ins = Instant::now();

            while let Some(i) = s.next().await {
                // println!("a = {:?}", i);

                if i == all - 1 {
                    println!("i = {}, time = {:?}", i, ins.elapsed());
                }
            }
        }
    });

    tokio::time::sleep(Duration::from_secs(100)).await;
}
