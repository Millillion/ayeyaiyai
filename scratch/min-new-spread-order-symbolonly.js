var calls = [];
var o = {};
Object.defineProperty(o, Symbol('foo'), { get: () => { calls.push("Symbol(foo)") }, enumerable: true });

new function(obj) {
  Object.keys(obj).length;
}({...o});
