## Prelude
All variables from this module are automatically in all modules

* ```print(value)```

    Prints ```value```

* ```eval(source)```

    Evaluates expression ```source``` in the context of the current module and returns it. Throws ```TypeError``` if ```source``` is not a string and ```CompileError``` if there is a compile error.

    Example: ```eval('1+1') //2```

* ```exec(source)```

    Executes expression ```source``` in the context of the current module. Throws ```TypeError``` if ```source``` is not a string and ```CompileError``` if ```source``` could not be compiled

    Example: ```eval('1+1') //2```

* ```spawn(fn)```

    Spawns a new task with the function ```fn``` and returns the created task

* ```spawn_link(fn)```

    Spawns a new task with the function ```fn``` which is linked with the current task and returns the created task. 

* ```join(tasks)```

    Waits for each task in ```tasks``` to complete. If any one task is killed, all tasks in ```tasks``` are killed and the error with which that task was killed is thrown.

