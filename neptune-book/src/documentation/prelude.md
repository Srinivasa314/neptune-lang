## Prelude
All variables from this module are automatically in all modules

* ```print(value)```

    Prints ```value```

* ```eval(source)```

    Evaluates expression ```source``` in the context of the current module and returns it. Throws ```TypeError``` if ```source``` is not a string and ```CompileError``` if ```source``` is not an expression or could not be compiled

    Example: ```eval('1+1') //2```

* ```exec(source)```

    Executes expression ```source``` in the context of the current module. Throws ```TypeError``` if ```source``` is not a string and ```CompileError``` if ```source``` could not be compiled

    Example: ```eval('1+1') //2```