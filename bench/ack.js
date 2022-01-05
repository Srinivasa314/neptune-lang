function Ack(M, N) {
    if (M === 0)
        return N + 1
    if (N === 0)
        return Ack(M - 1, 1)
    return Ack(M - 1, Ack(M, (N - 1)))
}

let time = performance.now()
console.log(Ack(3, 9))
console.log(performance.now() - time)