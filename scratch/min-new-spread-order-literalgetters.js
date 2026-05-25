var calls = [];
var o = { get z() { calls.push('z') }, get a() { calls.push('a') } };

new function(obj) {
  Object.keys(obj).length;
}({...o});
