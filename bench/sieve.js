function sieve(primes) {
    let n = primes.length - 1;
    const sqrt_n = Math.sqrt(n)
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
let arr = new Array(5_000_001).fill(true);
let time = performance.now()
console.log(sieve(arr))
console.log(performance.now() - time)
