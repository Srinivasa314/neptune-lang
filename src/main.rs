use neptune_lang::Neptune;

fn main() {
    let n = Neptune::new();
    println!("{:?}", n.run("1+1"));
}
