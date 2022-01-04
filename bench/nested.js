function loop() {
    let counter = 0
    for (let a = 0; a < 75; a++) {
        for (let b = 0; b < 75; b++) {
            for (let c = 0; c < 75; c++) {
                for (let d = 0; d < 75; d++) {
                    counter += 1
                }
            }
        }
    }
    return counter
}

let time = performance.now()
console.log(loop())
console.log(performance.now() - time)