function fib(n) {
    if (n < 2)
        return n;
    return fib(n - 1) + fib(n - 2);
}

let time = performance.now()
console.log(fib(33))
console.log(performance.now() - time)