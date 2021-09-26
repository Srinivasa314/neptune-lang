use neptune_lang::Neptune;

fn main() {
    let mut n = Neptune::new();
    n.exec(
        r"
        a=[0]
        a[0]=a
    ",
    )
    .unwrap();
    println!("{:?}", n.eval("a"));
}
