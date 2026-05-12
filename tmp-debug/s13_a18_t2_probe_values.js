this.callee = 0;
var b;

__obj = { callee: "a" };

function f() {
    with (arguments) {
        callee = 1;
        b = true;
        return arguments;
    }
}

result = f(__obj);

console.log(callee);
console.log(__obj.callee);
console.log(result.callee);
console.log(b);
console.log(this.b);
console.log(this.callee);
