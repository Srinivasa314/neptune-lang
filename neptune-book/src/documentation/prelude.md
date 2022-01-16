## Prelude

* ```print(value)```

Prints ```value```

* ```eval(source)```

Evaluates expression ```source``` and returns it. Throws ```TypeError``` if ```source``` is not a string and ```CompileError``` if ```source``` is not an expression or could not be compiled

Example: ```eval('1+1')//2```

* ```exec(source)```

Executes expression ```source```. Throws ```TypeError``` if ```source``` is not a string and ```CompileError``` if ```source``` could not be compiled

Example: ```eval('1+1')//2```