# Control flow

## Basic control flow
Like most languages Neptune lang supports if,if else and while statements. 
```
if x == 5{
    print('five')
}

if cond {
    print('if')
} else {
    print('else')
}

while x!==0 {
    x-=1
}
```

## For loop
The for statement is used to loop through the values in an iterator. Refer the Iterator subchapter in the documentation for more information.
```
for i in 0..10{
    print(i)
}
```

## Break and Continue
They are used to exit a loop early. Break exits the loop while continue starts a new iteration of a loop.

## Switch
The switch statement can be used instead of an if else ladder. Unlike an if else ladder the time taken to go to the required statement is independent of the number of cases. Constant literals must be used as cases of the switch statement. `default` is used to execute a statement if nothing is matched. The `or` keyword can be used in a case as shown below.

```
switch 1+1{
    1 or 2: print('1 or 2')
    default: print('other')
}
```
## Exception handling
Exceptions are used to indicate errors. They are raised using `throw`. They can be caught using a try catch block.
```
try{
    throw new Error('abc')
}catch e{
    print(e.message) //abc
}
```
An error object contains two important fields:
* message - A description of the error
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
 