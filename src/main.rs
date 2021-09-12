use neptune_lang::Neptune;

fn main() {
    let mut n = Neptune::new();
    println!("{:?}", n.eval("(0.0/0.0)"));
}
