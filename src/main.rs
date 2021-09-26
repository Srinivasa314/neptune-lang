use neptune_lang::Neptune;

fn main() {
    let mut n = Neptune::new();
    n.exec(
        r"
        fun fib(n){
            if(n<2){
                return n
            }else{
                return
            }
        }
    ",
    )
    .unwrap();
    println!("{:?}", n.exec("return 1"));
}
