## Map
* ```construct()```

Returns and empty map

Example: ```new Map()//Map {}```

* ```contains(key)```

Returns whether ```key``` is in the map

Example 
```
let map = Map{}
map.contains(2)//false
```

* ```remove(key)```

Removes the key from the map. Throws ```KeyError``` if the key is not present

Example
```
let map = Map{1:2,2:3}
map.remove(1)
map//Map{2:3}
```

* ```clear()```
Removes all keys from the map

Example
```
let map = Map{1:2,2:3}
map.clear()
map//Map{}
```

* ```len()```

Returns the number of keys in the map

Example
```
let map = Map{1:2,2:3}
map.len()//2
```

* ```keys()```

Returns an iterator to the keys of the map

Example
```
let map = Map{1:2,2:3}
map.keys().collect()//[1,2]
```
