use neptune_lang::Neptune;

fn main() {
    let mut n = Neptune::new();
    println!("{:?}", n.eval("1 and 2"));
}
