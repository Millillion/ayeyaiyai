var __map = { foo: "bar" };
var x = +"bar";
console.log(x);
console.log(x + 1);
__map.foo = x + 1;
console.log(__map.foo);
