# Classes and Objects

Classes are created using the class keyword
```
class C extends Base{
    construct(val){
        super.construct()
        this.x=val
    }
    method1(){
        return this.x
    }
}
```
Classes can inherit the methods of other classes using the extends keyword.If no parent class is given then it extends Object. The constructor is created by creating a method called construct. Objects of class C can be created by
```
let c = new C(7)
```
Methods of the parent class can be called by super. The this keyword represents the instance of the class. Properties can be get and set using the . operator or be using the [] operator.
```
let person = {name:'abc', age:29}
person.age+=1
person[@gender]=@male // same as person.gender=@male
```
Private properties begin with _. They cannot be accessed outside a method.

