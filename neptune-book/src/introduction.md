## Basic Example
```
fun sayHello(names){
    for name in names.iter(){
        print('Hello \(name)')
    }
}

sayHello(['abc','efg'])
```

## Syntax
* Semicolons are optional
* Blocks are declared using curly braces
* Single line comment - `// hello`
* Multi line comment - `/* hello */`

## Builtin classes
* Int 
* Float 
* Bool (true or false)
* String
* Map
* Symbol
* Range
* Null (null)

## Int
* Int literals can be hex - 0x,octal - 0o and can have underscores between them
* It is a 32 bit signed integer
* When an arithmetic operation overflows it throws ```OverflowException```

## Float
* It is a 64 bit IEEE 754 floating point number
* Float literals can contain underscores and use exponential notation

## Variable Declaration
* mutable - ```let var = expr```
* immutable - ```const var = expr```
* destructuring -  
    ```
    let {a,b,c} = expr
    // is equivalent to
    let a = expr.a
    let b = expr.b
    let c =expr.c
    ``` 

## Ranges
* The range `start..end` contains all values with `start` <= x < `end`

## Symbols
* They are like strings but two symbols with the same contents are always the same object
so comparing them is faster.
* Symbol literals start with @ eg:`@abc`

## String Literals
* Can start and end with ' or "
* They can be multi-line
* \t,\n,\r,\0 escape sequences can be used
* Unicode escape sequence:
    ```
    let sparklingHeart = "\u{1F496}" // 💖, Unicode scalar U+1F496
    ```
* String interpolation is done like ```\(expr)```


## Strings
* They are UTF-8 strings
* String slices: `"Hello, 世界"[7..10] //"世"`
* To convert a value to string ```val.toString()```

## Operators
* basic operators: +, -, *, /, %, ~, <, <=, >, >=
* Strict equality ```====```: Like normal equality but
    * `Int`s and `Float`s that contain the same value are unequal eg. 1!==1.0
    * NaN===NaN
    * 0.0!==-0.0
* Instead of `a=a+1`, `a+=1` can be used.
* false and null are falsey others are truthy
* The `!` operator gives false if the value is truthy and true if the value is falsey
* The `and` and `or` operator are used on two conditions. They are short-ciruiting i.e They dont compute the second value if the first is falsey/truthy.
* `~` is used to concat two strings

## Maps
```Map {key:value,...}```
Values can be get and set using the `[]` operator

## Object literals
```{prop:val,...}```
Objects constructed by object literals are of the class `Object`


## Arrays
```[1,2,3]```
Array slices: ```[1,2,3,4][1..3] //[2,3]```

## Functions
```
fun function_name(arg1,arg2,...){
    function_body
}
```
Anonymous function:
```
|arg1,arg2,...|expression
```
or
```
|arg1,arg2,...|{statements}
```
All functions can capture variables
```
fun makeCounter(){
    let count = 0
    return ||{count+=1;return count}
}
let counter = makeCounter()
counter() //1
counter() //2
```

## Classes
```
class C extends Base{
    construct(val){
        super.construct()
        this.x=val
    }
    method1(){
        return this.x
    }
}
```
* If extends is not given it extends Object
* Constructor is construct
* To create an instance : ```new C(val)```
* Properies can be get and set using the `.` operator

## Control flow
```
if condition{
    statements
}

while condition{
    statements
}

for var in iterator{
    statements
}
```
* break and continue can be used

## Modules
Variables declared at the top  level scope are local to each module. To share them across
modules they must be exported and then imported by the other module.

To export 
```
//a.np
export fun f(){}
export class C{}
export let x=0
```
To import
```
//b.np
const a = import("./a.np")
a.x //Module variables too can be accessed by the dot operator
const {C,f} = import("./a.np")
```


## Exception handling

Throwing exceptions
```
throw new Error('')
```

Try-catch block
```
try{
    throw new Error('abc')
}catch e{
    print(e.message) //abc
}
```

An error object contains two important fields:
* message
* stack - It contains the stack trace

Error classes can be created by extending the class `Error`. For example, this is how the
class `TypeError` is defined in the standard library.

```
export class TypeError extends Error {
    construct(message) {
        super.construct(message)
    }
}
```
 