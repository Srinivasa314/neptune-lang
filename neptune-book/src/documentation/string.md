## String
* ```construct()```

    Returns an empty string

    Example ```new String() //''```

* ```find(str)```

    Returns the position of the first occurence of ```str```. Returns -1 if ```str``` is not present

    Example: ```'abc'.find('bc') //1```

* ```replace(from,to)```

    Returns a new string with all occurences of ```from``` replaced by ```to```

    Example: ```'abc'.replace('bc','xyz') //'axyz'```

* ```chars()```

    Returns an iterator to the characters of the string

    Example: ```'abc'.chars().collect() //['a','b','c']```
