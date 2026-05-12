var __PLANT = "flower";
function __PROTO() {}
__PROTO.type = __PLANT;
function __FACTORY() {}
__FACTORY.prototype = __PROTO;
var __rose = new __FACTORY();
console.log(1);
try {
  console.log(2);
  __rose();
  console.log(3);
  throw new Test262Error("call did not throw");
} catch (e) {
  console.log(4);
  console.log(e instanceof TypeError);
  console.log(e instanceof Error);
}
console.log(5);
