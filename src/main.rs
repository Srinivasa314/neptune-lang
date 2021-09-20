use neptune_lang::Neptune;

fn main() {
    let mut n = Neptune::new();
    //n.exec("x=0;for i in 1 to 11{x+=i}").unwrap();
    //println!("{:?}", n.eval("x"));

    //n.exec("g=0;for i in 0 to 100_000_000{g+=2}").unwrap();
    //println!("{:?}", n.eval("g"));
    n.exec("for i in 0 to 100_000_000{}").unwrap();
}
