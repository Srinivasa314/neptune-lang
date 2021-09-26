use neptune_lang::Neptune;

fn main() {
    let mut n = Neptune::new();
    n.exec(
        r"
        fun fib(n){
            if(n<2){
                return n
            }else{
                return fib(n-1)+fib(n-2)
            }
        }
    ",
    )
    .unwrap();
    println!("{:?}", n.exec("fib(30)"));
}
