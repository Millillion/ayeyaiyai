function Test262Error(message) {
  this.name = "Test262Error";
  this.message = message || "";
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
  throw new Test262Error(message || "sameValue failed");
}

function __ayyAssertThrows(expectedErrorConstructor, func, message) {
  try {
    func();
  } catch (thrown) {
    if (thrown.constructor !== expectedErrorConstructor) {
      throw new Test262Error(message || "wrong error");
    }
    return;
  }
  throw new Test262Error(message || "missing throw");
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

try {
  __assertSameValue(C.fieldAccess(), 'outer class');
  console.log('field ok');
  let b = new C.B();
  C.B.access(b);
  console.log(b._v);
  __assertSameValue(b._v, 'inner class');
  console.log('setter ok');
  __ayyAssertThrows(TypeError, function() {
    C.B.access(C);
  }, 'accessed private setter from an arbritary object');
  console.log('throws ok');
} catch (e) {
  console.log(e.name);
  console.log(e.message);
}
