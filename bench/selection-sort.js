function sortDescending(arr) {
    let len = arr.length
    for (let i = 0; i < len; i++) {
        for (let j = i + 1; j < len; j++) {
            if (arr[i] < arr[j]) {
                let t = arr[i]
                arr[i] = arr[j]
                arr[j] = t
            }
        }
    }
}

let time = performance.now()
let arr = new Array(4000)
for (let i = 0; i < arr.length; i++) {
    arr[i] = i
}
sortDescending(arr)
console.log(arr[0])
console.log(performance.now() - time)