function collatz(x) {
    while (x > 1) {
        if (x % 2 === 0) {
            x /= 2
        } else {
            x = x * 3 + 1
        }
    }
}

for (let i = 1; i < 100000; i++) {
    collatz(i)
}
