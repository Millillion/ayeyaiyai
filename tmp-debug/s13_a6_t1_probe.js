var __FOO = "fooValue";
var __BAR = "barValue";
var __func = function(arg) {
  this.foo = arg;
  return 0;
  this.bar = arguments[1];
};
var __obj = new __func(__FOO, __BAR);
console.log(__obj.foo);
console.log(__obj.bar);
console.log(typeof __obj);
console.log(__obj === 0);
