var __MONSTER = "monster";
var __PREDATOR = "predator";

function __PROTO() {}

console.log(1);
try {
  __PROTO.type = __MONSTER;
  console.log(2);
} catch (e) {
  console.log(10);
  throw 10;
}

function __FACTORY() {}

console.log(3);
__FACTORY.prototype = __PROTO;
console.log(4);

var __monster = new __FACTORY();
console.log(5);
console.log(__PROTO.isPrototypeOf(__monster));
console.log(__monster.type);
