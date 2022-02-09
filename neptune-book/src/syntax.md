# Basic Syntax
## Comments
* Comments can be single line or multi-line
    * Single line comment - `// hello`
    * Multi line comment - `/* hello */`
* Comments can nest
```
/* /* nested */ */
```

## Identifiers
They can begin with _ or a letter and can contain letters, numbers and underscores.

## Statements
Statements may be separated by semicolons but it is not compulsory

## Blocks
They are declared using curly brackets
```
{
    let a = 'hello'
    print(a)
}
```

## Int literals
* Int literals can be hex - 0x,octal - 0o 
* They can have underscores between them
* Example: 0xdead_beef

## Float literals
* They can contain underscores 
* Exponential notation is also allowed
* Example: 1.2e9

## String Literals
* They can start and end with ' or "
* They can be multi-line
* \t,\n,\r,\0 escape sequences can be used
* Unicode escape sequences begin with \u
    ```
    let sparklingHeart = "\u{1F496}" // 💖, Unicode scalar U+1F496
    ```
* They also allow interpolation using \ followed by an expression in parentheses
    ```
    let greeting = 'Hello \(name)!'
    ```