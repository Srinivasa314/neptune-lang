use neptune_lang::Neptune;

fn main() {
    let n = Neptune::new();
    n.exec("let a=1+1").unwrap();
}
