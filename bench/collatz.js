let max_steps = 0;

function collatz(x) {
    let steps = 0;
    while (x > 1) {
        steps++;
        if (x % 2 === 0) {
            x /= 2
        } else {
            x = x * 3 + 1
        }
    }
    if (steps > max_steps)
        max_steps = steps;
}

let time = performance.now()
for (let i = 1; i < 100000; i++) {
    collatz(i)
}
console.log(max_steps)
console.log(performance.now() - time)