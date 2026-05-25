var __obj = {};
console.log("c1", Object.prototype.isPrototypeOf(__obj));

var protoObj = {};
function FooObj() {}

var obj__ = new FooObj;
console.log("c21", Object.prototype.isPrototypeOf(obj__));
console.log("c22", FooObj.prototype.isPrototypeOf(obj__));
console.log("c23", protoObj.isPrototypeOf(obj__));

FooObj.prototype = protoObj;
console.log("c24", protoObj.isPrototypeOf(obj__));

var __foo = new FooObj;
console.log("c31", Object.prototype.isPrototypeOf(__foo));
console.log("c32", FooObj.prototype.isPrototypeOf(__foo));
console.log("c33", protoObj.isPrototypeOf(__foo));
