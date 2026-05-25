var calls = [];
var o = { get z() { calls.push('z') } };

new function(obj) {
  Object.keys(obj).length;
}({...o});
