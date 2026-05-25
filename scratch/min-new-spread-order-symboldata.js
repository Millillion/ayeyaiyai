var o = {};
Object.defineProperty(o, Symbol('foo'), { value: 1, enumerable: true });

new function(obj) {
  Object.keys(obj).length;
}({...o});
