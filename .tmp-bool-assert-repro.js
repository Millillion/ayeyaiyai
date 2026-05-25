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

function f() {
  return false;
}

__assertSameValue(f(), false);
console.log("ok");
