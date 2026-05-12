var a = 1;

var __obj = { a: 2 };

with (__obj) {
    result = (function () { return a; })();
}

console.log(result);
console.log(a);
console.log(__obj.a);
