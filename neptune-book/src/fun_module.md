# Functions and Modules

## Functions
Functions are can be declared as 
* named functions:
```
fun add(a,b){
    return a+b
}
```
* anonymous functions:
```
let add = |a,b|a+b
//or
add = |a,b|{return a+b}
```

Functions can capture variables
```
fun makeCounter(){
    let count = 0
    return ||{count+=1;return count}
}
let counter = makeCounter()
counter() //1
counter() //2
```

## Modules
Programs can be broken down into small pieces called module. A new module is created by creating a new file and can be imported by using the import function. An embedder can decide how to resolve paths while importing.


To share variables across modules they must be exported and then imported by the other module.
A module variable that is exported can be accessed using the . operator.
```
import('math').PI //3.1415926535898
```
If a module is imported multiple times only one copy of the module will be created and it will be reused.

Example:
```
//a.np
export fun f(){}
export class C{}
export let x=0
```
```
//b.np
const {x,C,f} = import("./a.np")
```
There are many inbuilt modules that can be used to generate random numbers, evaluate expressions at runtime,etc. They can be explored using the documentation.