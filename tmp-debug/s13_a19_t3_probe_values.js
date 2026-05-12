var a = 1;

var __obj = { a: 2 };

try {
    with (__obj) {
        var __func = function () { return a; };
        throw 3;
    }
} catch (e) {
}

result = __func();

console.log(result);
console.log(a);
console.log(__obj.a);
console.log(typeof __func);
console.log(this.__func === __func);
