# Variables and Equality

## Scope
There are two types of scope: block scope and module scope. Variables declared in a block can be accessed within that block. Variables cannot be redeclared in a block but can be shadow a declaration from a higher scope. Variables declared at the top level have module scope.

## Variable Declaration
Variables can be declared using `let` or `const`. Variables declared as const cannot be reassigned. 
```
const VERSION_STRING = '1.0.0' 
```
Variables can also be declared using 'destructuring declaration'
```
let point = {x:1, y:2}
let {x,y} = point // x is 1 and y is 2
```

## Equality and Strict Equality
Variables can be tested for equality using the `==` operator. The `==` can be used for most situations and strict equality(`===`) is different in the following ways.
* 1(`Int`) and 1.0(`Float`) are not strictly equal
* -0.0 and 0.0 are not strictly equal 
* NaN and NaN are strictly equal
Strict equality is used by `Map`.
