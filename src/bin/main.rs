use std::{error::Error, time::Instant};

use futures::{future::ready, StreamExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut p = futures::stream::iter(0..100_000_000)
        .filter(|i| {
            //
            if *i % 2 == 0 {
                ready(true)
            } else {
                ready(false)
            }
        })
        .map(|i| Result::<i32, Box<dyn Error>>::Ok(i));

    let mut x = 0;

    let ins = Instant::now();

    while let Some(_) = p.next().await {
        x += 1;
    }

    println!("x = {}, time = {:?}", x, ins.elapsed());

    // let (tx, mut rx) = futures::channel::mpsc::channel(128);

    // let stream = futures::stream::iter(0..100_000_000);

    // tokio::spawn(stream.map(|i| Ok(i)).forward(tx));

    // let mut x = 0;

    // let ins = Instant::now();

    // while let Some(_) = rx.next().await {
    //     x += 1;
    // }

    // println!("x = {}, time = {:?}", x, ins.elapsed());

    // let s = Arc::new(Mutex::new(0));

    // let ins = Instant::now();

    // let mut s = s.lock().await;
    // for i in 0..100_000_000 {
    //     if i % 2 == 0 {
    //         *s += 1;
    //     } else {
    //         *s += 2;
    //     }
    // }

    // println!("time = {:?}", ins.elapsed());

    tokio::time::sleep(std::time::Duration::from_secs(100)).await;
    Ok(())
}
