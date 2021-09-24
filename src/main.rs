use neptune_lang::Neptune;

fn main() {
    let mut n = Neptune::new();
    n.exec(
        r"
        let head=null
        for j in 0 to 100000{
            head={@next:head}
        }
    ",
    )
    .unwrap();
}
