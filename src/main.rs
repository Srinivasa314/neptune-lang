use neptune_lang::Neptune;

fn main() {
    let n = Neptune::new();
    println!("{:?}", n.run("let x=[1,2,];x[0]=4;x[0]"));
}
