var callee = 0, b;
var __obj = { callee: "a" };
result = (function () {
    with (arguments) {
        callee = 1;
        b = true;
    }
    return arguments;
})(__obj);
console.log(callee);
console.log(__obj.callee);
console.log(result.callee);
console.log(b);
console.log(this.b);
