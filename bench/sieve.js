function sieve(n) {
    const sqrt_n = Math.sqrt(n)
    let primes = new Array(n + 1).fill(true);
    for (let p = 2; p <= sqrt_n; p++) {
        if (primes[p] === true) {
            for (let i = p * p; i <= n; i += p)
                primes[i] = false
        }
    }

    let count = 0
    for (let i = 2; i <= n; i++) {
        if (primes[i] === true)
            count++
    }
    return count
}
let time = performance.now()
console.log(sieve(1_000_000))
console.log(performance.now() - time)
