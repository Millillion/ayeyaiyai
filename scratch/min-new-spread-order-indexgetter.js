var calls = [];
var o = {};
Object.defineProperty(o, 1, { get: () => { calls.push(1) }, enumerable: true });

new function(obj) {
  Object.keys(obj).length;
}({...o});
