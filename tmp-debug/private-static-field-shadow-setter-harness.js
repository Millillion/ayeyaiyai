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
  if (__sameValue(actual, expected)) {
    return;
  }
  console.log("sameValue fail");
  console.log(message);
  console.log(actual);
  console.log(expected);
  throw new Test262Error(message ?? "sameValue");
}

function __ayyAssertThrows(expectedErrorConstructor, func, message) {
  try {
    func();
  } catch (thrown) {
    if (thrown.constructor !== expectedErrorConstructor) {
      console.log("wrong throw");
      console.log(thrown.name);
      throw new Test262Error(message ?? "wrong throw");
    }
    return;
  }
  console.log("no throw");
  throw new Test262Error(message ?? "no throw");
}

class C {
  static #m = 'outer class';

  static fieldAccess() {
    return this.#m;
  }

  static B = class {
    set #m(v) { this._v = v; }

    static access(o) {
      o.#m = 'inner class';
    }
  }
}

console.log("before field");
__assertSameValue(C.fieldAccess(), 'outer class', "field");
console.log("after field");

let b = new C.B();

console.log("before setter");
C.B.access(b);
console.log("after setter");
console.log(b._v);
__assertSameValue(b._v, 'inner class', "setter");
console.log("after setter assert");

__ayyAssertThrows(TypeError, function() {
  C.B.access(C);
}, 'accessed private setter from an arbritary object');
console.log("after throws assert");
