# Tasks and Channels

Tasks are lightweight units of concurrent execution. Tasks are created by the `spawn()` or `spawn_link()` functions . A task is killed when an uncaught exception is thrown but can also be killed by the `kill()` method. If a task fails it does not crash other tasks. `spawn_link()` links the newly created task with the task that created it. If two tasks are linked then one if one of them dies then the other will be killed. Tasks can be manually linked too.  Tasks can also be given names for debugging purposes. Channels can be used to `send()` and `recv()` messages. If a task is waiting on a channel, it is woken up once the message is received. The `join` function can be used to wait for multiple tasks to complete. It throws an exception and if one of them fails. Errors from multiple tasks can be handled gracefully using the `monitor()` method of `Task`. An example is given below.

```
class MySupervisor{
	construct(){
		this.monitorChan=new Channel()
		this.children=new Map()
	}

    // This method can be called even if it is running	
	// restartPolicy can be
    // @permanent: it should be restarted if it exits
    // @transient: it should be restarted if it exits unsuccessfully
    // @temporary: it shouldnt be restarted
	spawn(f,restartPolicy){
		let task=spawn(f)
		this.children[task]={f,restartPolicy}
		task.monitor(monitorChan)
	}
	
	run(){
		while(true){
			let task = monitorChan.recv();
			let childEntry = this.children[task]
			switch childEntry.restartPolicy{
				@permanent:this.spawn(childEntry.f,@permanent)
				@transient:if task.status === @killed{
					this.spawn(childEntry.f,@transient)
				}
				//do nothing if @temporary
			}
		}
	}
}
```

All methods of `Task` can be viewed in the documentation section of the book.