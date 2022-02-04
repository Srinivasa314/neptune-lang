## Module 'random'

* ```random()```

    Returns a random ```Float``` in the range 0.0 to 1.0

Example: ```random() //0.71189203412849```

* ```range(start,end)```

    Returns a random ```Int``` in the range ```start``` to ```end```. Throws ```TypeError``` if ```start``` or ```end``` is not an ```Int```

    Example: ```range(1,10) //6```

* ```shuffle(array)```

    Shuffles ```array```. Throws ```TypeError``` if array is not an ```Array```

    Example: 
    ```
    let arr = [1,2,3,4,5]
    shuffle(arr)
    arr //[4,2,1,5,3]
    ```