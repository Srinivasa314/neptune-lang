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
 