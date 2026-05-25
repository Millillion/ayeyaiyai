var set = Object.getOwnPropertyDescriptor({ set m(x = 42) {} }, 'm').set;

if (set.length !== 0) {
  throw new Error('length value');
}
