fn main() {
    let mut list = Vec::with_capacity(128);

    for i in list.iter_mut() {
        *i = rand::random::<u8>();
    }

    println!("{:?}", list)
}
