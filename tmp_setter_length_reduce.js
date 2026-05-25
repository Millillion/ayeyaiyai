var set = Object.getOwnPropertyDescriptor({ set m(x = 42) {} }, 'm').set;
var desc = Object.getOwnPropertyDescriptor(set, 'length');

if (set.length !== 0) {
  throw new Error('length value');
}
if (desc.value !== 0) {
  throw new Error('descriptor value');
}
if (desc.writable !== false) {
  throw new Error('descriptor writable');
}
if (desc.enumerable !== false) {
  throw new Error('descriptor enumerable');
}
if (desc.configurable !== true) {
  throw new Error('descriptor configurable');
}
