function make_tree(depth){
    if (depth==0) {
        return [null,null]
    }
    depth -= 1
    return [make_tree(depth), make_tree(depth)]
}

function check_tree(node){
    let left=node[0]
    let right=node[1]
    if(left){
        return 1 + check_tree(left) + check_tree(right)
    }
    return 1
}

let min_depth = 4
let max_depth = 15
let stretch_depth = max_depth + 1
console.log(`stretch tree of depth ${stretch_depth}\t check:${check_tree(make_tree(stretch_depth))}`)

let long_lived_tree = make_tree(max_depth)

let iterations = 32768//2**15

let depth=min_depth

while(depth < stretch_depth){
    let check = 0
    for(let i=0;i<iterations;i++){
        check += check_tree(make_tree(depth))
    }

    console.log(`${iterations}\t trees of depth ${depth}\t check:${check}`)
    iterations /= 4
    depth+=2
}

console.log(`long lived tree of depth ${max_depth} \t check ${check_tree(long_lived_tree)}`)
