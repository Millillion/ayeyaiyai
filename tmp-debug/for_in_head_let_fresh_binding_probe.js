var fns = {};
var obj = Object.create(null);
obj.a = 1;
obj.b = 1;
obj.c = 1;

for (let x in obj) {
  fns[x] = function() { return x; };
}

console.log(typeof fns.a);
console.log(fns.a());
console.log(typeof fns.b);
console.log(fns.b());
console.log(typeof fns.c);
console.log(fns.c());
