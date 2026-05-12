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

function __assertSameValue(actual, expected, message) {
  console.log("assert");
  console.log(actual);
  console.log(expected);
  try {
    if (__sameValue(actual, expected)) {
      return;
    }
  } catch (error) {
    throw new Test262Error((message ?? "") + " (_isSameValue operation threw) " + error);
  }
  throw new Test262Error("bad");
}

__assertSameValue(2, 2);
