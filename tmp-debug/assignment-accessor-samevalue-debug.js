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

let x = 0;

let C = class {
  get [x = 1]() {
    return 2;
  }

  set [x = 1](v) {
    return 2;
  }

  static get [x = 1]() {
    return 2;
  }

  static set [x = 1](v) {
    return 2;
  }
};

let c = new C();

__assertSameValue(c[x = 1], 2);
__assertSameValue(c[x = 1] = 2, 2);
__assertSameValue(C[x = 1], 2);
__assertSameValue(C[x = 1] = 2, 2);
__assertSameValue(c[String(x = 1)], 2);
__assertSameValue(c[String(x = 1)] = 2, 2);
__assertSameValue(C[String(x = 1)], 2);
__assertSameValue(C[String(x = 1)] = 2, 2);
__assertSameValue(x, 1);
