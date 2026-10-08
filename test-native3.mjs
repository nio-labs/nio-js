/** @native */
function json_test(str) {
    return JSON.parse(str).padding.length;
}
console.log(json_test('{"padding":"xxx"}'));
