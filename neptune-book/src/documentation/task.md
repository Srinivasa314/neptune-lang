## Task

* ```kill(exception)```

    Kills the task with ```exception```

* ```name()```

    Returns the name of the task

* ```setName(name)```

    Sets the name of the task as ```name```

* ```monitor(chan)```

    Sends the task to chan when the task has completed or is killed

* ```link(task2)```

    Links the task to task2. If task2 is killed,it is killed and when it is killed, task2 is killed.

* ```status()```

    Returns the status of the task (@running, @finished or @killed)

* ```getUncaughtException()```

    Returns the uncaught exception of the task or null if there is no uncaught exception