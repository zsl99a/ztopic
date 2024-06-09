use std::hash::{DefaultHasher, Hash, Hasher};

fn main() {
    let data = String::from("hello world");

    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);

    let hash = hasher.finish();

    println!("{}", hash);
}
