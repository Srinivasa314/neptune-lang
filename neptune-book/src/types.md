# Types
All values have a type and the type is associated with a class. The primitive types are listed below. These classes have many useful methods that can be seen in the documentation section of the book.

## Object
All classes inherit Object.Objects can be directly created using object literals
```
let point = {x:1, y:2}
```
The point has two properties x and y. 

## Int 
They are 32 bit signed integers. They can be manipulated using the operators `+`, `-`, `*`, `-`, `%`, `<`, `<=`, `>`, `>=`.Whenever any arithmetic operation overflows `OverflowError` is thrown.
Shorthand assignment operators (+=,-=,etc.) can be used too.

## Float 
They are IEEE-754 double precision floating point numbers. Unlike `Int` errors in arithmetic operations result in `NaN`.

## Bool
There are two possible values: `true` and `false`. They can be manipulated using `!`(not),`and` and `or`. The latter two are short-circuiting(they dont compute the second value if the first is falsey/truthy). They can be used like the ternary operator in C. All values except false and null are considered truthy.
```
let result = condition and 'true' or 'false'
```

## String
They are a sequence of characters encoded in UTF-8. They cannot be directly indexed but can be indexed using a range.
```
"Hello, 世界"[7..10] //"世"
```
The characters of a string can be got using the `chars` method which returns an iterator.
Strings can be concated using the tilde `~` operator.
Other types can be converted to strings using the `toString` method.

## Array
They are a list of values. They can be indexed using the `[]` operator. They can grow or shrink using the `push` and `pop` methods. The elements can be iterated using the `iter` method.
```
let a = [1,'hello',2]
```
Arrays can also be sliced (indexed by range) like strings.
```
[1, 2, 3, 4][1..3] //[2, 3]
```

## Map
They are hashmaps that indexed using any type. The keys of a map can be iterated using the `keys` method but the order of keys is not defined.
```
let m = Map{@a:1,2:false}
m[2] //false
m["abc"]=1.5
```

## Symbols
They are like strings but two symbols with the same contents are internally the same object. Comparing symbols are much faster than strings. They are used to store the names of properties and methods for quick access
```
let cardColor = @red
```

## Range
It denotes a range of integers. The range `start..end` contains all values with `start` <= x < `end`. It is an iterator. 
```
let r = 0..5
r.collect() //[0, 1, 2, 3, 4,]
```

## Null
It is used to denote nothing
```
let linkedList = {next:null}
```
