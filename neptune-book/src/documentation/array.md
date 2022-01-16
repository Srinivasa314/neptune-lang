## Array

* ```construct(length,value)```

Returns an array of length ```length``` filled with ```value```. Throws ```Error``` if length is negative and ```TypeError``` if ```length``` is not an ```Int```

Example: ```new Array(3,0)//[0,0,0]```

* ```push(value)```

Appends ```value``` to its end

Example
```
let arr = [1,2]
arr.push(3)
arr[1,2,3]
```

* ```pop()```

Removes the last element and returns it. Throws ```IndexError``` if it is empty

Example
```
let arr = [1,2]
arr.pop()//2
```

* ```len()```

Returns the length of the array

Example
```
let arr = [1,2]
arr.len()//2
```

* ```insert(position,value)```

Inserts ```value``` at ```position```. Throws IndexError if ```position``` is greater than its length and ```TypeError``` if ```position``` is not an ```Int```

Example
```
let arr = [1,2]
arr.insert(0,3)
arr//[3,1,2]
```

* ```remove(position)```

Removes the value at ```position```. Throws IndexError if ```position``` is greater than or equal to its length and ```TypeError``` if ```position``` is not an ```Int```

Example
```
let arr = [1,2]
arr.remove(0)
arr//[2]
```

* ```clear()```

Removes all elements of the array

Example
```
let arr = [1,2]
arr.clear()
arr//[]
```

* ```iter()```

Returns an iterator to the elements of the array

Example: ```[1,2,3].iter().collect()//[1,2,3]```

* ```sort(compare)```

Sorts the array comparing by function ```compare```.

Example
```
let arr = [4,1,3,2]
arr.sort(|x,y|x<y)
arr//[1,2,3,4]
```