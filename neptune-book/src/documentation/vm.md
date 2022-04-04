## Module 'vm'

* ```disassemble(fn)```

    Returns a string containing the bytecode of fn. Throws ```TypeError``` if fn is not a ```Function``` or if fn is a native function

* ```gc()```

    Runs garbage collection

* ```generateStackTrace(depth)```

    Returns the stack trace at depth ```depth```. Throws ```TypeError``` if depth is not an ```Int```

* ```ecall(op,args)```

    Calls EFunc ```op``` with argument ```args```. Throws TypeError if ```op``` is not a symbol and ```Error``` if ```op``` is not an EFunc

* ```currentTask()```

    Returns the current task
