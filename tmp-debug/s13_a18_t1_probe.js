var callee = 0, b;
var __obj = { callee: "a" };
result = (function () {
    with (arguments) {
        callee = 1;
        b = true;
    }
    return arguments;
})(__obj);
console.log(callee === 0);
console.log(__obj.callee === "a");
console.log(result.callee === 1);
console.log(this.b === true);
