/** @native */
function arr_test() {
    const arr = Array.from({ length: 5 }, (_, i) => i);
    return arr.map(x => x * 3).reduce((a, b) => a + b, 0);
}
console.log(arr_test());
