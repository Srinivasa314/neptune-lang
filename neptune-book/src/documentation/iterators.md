## Iterators

* ```each(fn)```

Calls function ```fn``` for all items in the iterator

Example:
```
let a=[]
[1,2,3].iter().each(|x|a.push(x))
a//[1,2,3]
```

* ```all(fn)```

Returns if for all items in the interator ```fn(item)``` is truthy

Example: ```[1,2,3].iter().all(|x|x<10)//true```

* ```any(fn)```

Returns if for any item in the iterator ```fn(item)``` is truthy

Example: ```[1,2,3].iter().any(|x|x==1)//true```

* ```map(fn)```

Returns a new iterator whose items are ```fn(item)``` of the items of this iterator

Example: ```[1,2,3].map(|x|x+1).collect()//[2,3,4]```

* ```filter(fn)```

Returns a new iterator whose items are those items of this iterator for which ```fn(item)``` is truthy

Example: ```[1,2,3].filter(|x|x%2==1).collect()//[1,3]```

* ```collect()```

Collects all items of the iterator into an array

Example: ```[1,2,3].iter().collect()//[1,2,3]```

* ```count()```

Returns the number of items of the iterator

Example: ```[1,2,3].iter().count()//3```

* ```reduce(fn,initial)```

Applys ```fn```, a function of two arguments cummulatively to the items of the iterator starting with initial

Example: ```[1,2,3].iter().reduce(|x,y|x+y,0)//6```