var n = {};
var m = n;
console.log(typeof m);
function populateAge(person) {
  person.age = 50;
}
populateAge(m);
console.log(n.age);
console.log(m.age);
