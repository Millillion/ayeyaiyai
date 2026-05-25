var o = { get z() { return 1 } };

new function(obj) {
  Object.keys(obj).length;
}({...o});
