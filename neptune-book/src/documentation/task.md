## Task

* ```kill(exception)```

    Kills the task with ```exception```

* ```name()```

    Returns the name of the task

* ```setName(name)```

    Sets the name of the task as ```name```

* ```monitor(chan)```

    When the task is completed or killed it sends the task to the channel `chan`.

* ```link(task2)```

    Links the task to task2. If task2 is killed,it is killed and when it is killed, task2 is killed.

* ```status()```

    Returns the status of the task (@running, @finished or @killed)

* ```getUncaughtException()```

    Returns the uncaught exception that killed the task or null if there is no uncaught exception