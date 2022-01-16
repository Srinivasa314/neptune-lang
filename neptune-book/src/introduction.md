# Chapter 1
## Basic Example
```
fun sayHello(names){
    for name in names{
        print('Hello \(name)')
    }
}

sayHello(['abc','efg'])
```

## Basic datatypes
* Int
* Float - 64 bit double
* Bool
* String
* Map
* Symbol
* Range
* Null (null)

## Int
* Can be hex - 0x,octal - 0o
* 32 bit signed integer
* When an arithmetic operation overflows it throws ```OverflowException```


## Variables
* mutable - ```let var=expr```
* immutable - ```const var=expr```
* destructuring - ```let {a,b,c}=expr```
## Comments
Single line - //
Multi line - /* */

## Ranges
* ```start..end``` (from start to end-1)

## Symbols
* @symbolname

## Strings
* Start and end with ' or "
* Can be multi-line
* Templates using ```\()```
* String slices ```str[range]```
* To convert to string ```val.toString()```
* To concat ```str1~str2```

## Operators
```+,-,*,/,%,~(concat),*,*=,<,<=,!```
Normal equality ```==```
Strict equality ```====```
```+=,-=,*=,/=```
* false and null are falsey others are truthy

## Maps
```Map {key:value,...}```

## Object literals
```{prop:val,...}```


## Arrays
```[1,2,3]```
* Array slices ```array[range]```

## Blocks
```
{
    statements
}
```
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
* All functions can capture variables
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
To export 
```
export fun f(){}
export class C{}
export let x=0
```
To import
```
import("module")
```

## Exception handling
Throwing exceptions
```
throw new Exception('')
```
Try-catch block
```
try{
    statements
}catch var{
    statements
}
```

## Iterators
* Those that support hasNext() and next()
* array.iter(),map.keys(),str.chars(),ranges give iterators
