use neptune_lang::Neptune;

fn main() {
    let mut n = Neptune::new();
    n.exec("if true{a=1}").unwrap();
    println!("{:?}", n.eval("a"));
}
