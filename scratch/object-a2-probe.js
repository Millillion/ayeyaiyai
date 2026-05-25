var x = true;
var object = { prop: x };
console.log("1", object.prop === x);

x = new Boolean(true);
object = { prop: x };
console.log("2", object.prop === x);

x = 1;
object = { prop: x };
console.log("3", object.prop === x);

x = new Number(1);
object = { prop: x };
console.log("4", object.prop === x);

x = "1";
object = { prop: x };
console.log("5", object.prop === x);

x = new String(1);
object = { prop: x };
console.log("6", object.prop === x);

x = undefined;
object = { prop: x };
console.log("7", object.prop === x);

x = null;
object = { prop: x };
console.log("8", object.prop === x);

x = {};
object = { prop: x };
console.log("9", object.prop === x);

x = [1, 2];
object = { prop: x };
console.log("10", object.prop === x);

x = function() {};
object = { prop: x };
console.log("11", object.prop === x);

x = this;
object = { prop: x };
console.log("12", object.prop === x);
