function Test262Error(message) {
  this.name = "Test262Error";
  this.message = message ?? "";
}

function __sameValue(left, right) {
  if (left === right) {
    return left !== 0 || 1 / left === 1 / right;
  }
  return left !== left && right !== right;
}

function __assertSameValue(actual, expected) {
  if (__sameValue(actual, expected)) {
    return;
  }
  throw new Test262Error("failed");
}

function assert(mustBeTrue, message) {
  if (mustBeTrue === true) {
    return;
  }
  throw new Test262Error(message);
}

globalThis.assert = assert;
assert.sameValue = __assertSameValue;

class Class {
  #field;

  static isNameIn(value) {
    return #field in value;
  }
}

console.log("actual", Class.isNameIn({}));
console.log("strict", Class.isNameIn({}) === false);
__assertSameValue(Class.isNameIn({}), false);
console.log("actual2", Class.isNameIn(new Class()));
__assertSameValue(Class.isNameIn(new Class()), true);
console.log("ok");
