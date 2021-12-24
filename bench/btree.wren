var make_tree
make_tree=Fn.new{|depth|
    if(depth==0) return [null,null]
    depth = depth-1
    return [make_tree.call(depth), make_tree.call(depth)]
}
var check_tree
check_tree=Fn.new{|node|
    var left=node[0]
    var right=node[1]
    if(left) return 1 + check_tree.call(left) + check_tree.call(right)
    return 1
}

var min_depth = 4
var max_depth = 14
var stretch_depth = max_depth + 1
System.print("stretch tree of depth %(stretch_depth)\t check:%(check_tree.call(make_tree.call(stretch_depth)))")

var long_lived_tree = make_tree.call(max_depth)

var iterations = 16384//2**14

var depth=min_depth

while (depth < stretch_depth){
    var check = 0
    for(i in 0..iterations){
        check = check + check_tree.call(make_tree.call(depth))
    }

    System.print("%(iterations)\t trees of depth %(depth)\t check:%(check)")
    iterations = iterations/4
    depth=depth+2
}

System.print("long lived tree of depth %(max_depth) \t check %(check_tree.call(long_lived_tree))")
