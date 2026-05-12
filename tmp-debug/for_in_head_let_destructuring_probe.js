var obj = Object.create(null);
var value;

obj.key = 1;

for (let [x] in obj) {
  value = x;
}

console.log(typeof x);
console.log(value);
