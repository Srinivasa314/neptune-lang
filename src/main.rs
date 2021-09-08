use neptune_lang::Neptune;

fn main() {
    let n = Neptune::new();
    println!("{:?}", n.eval(r#"a+2000000000"#));
}
