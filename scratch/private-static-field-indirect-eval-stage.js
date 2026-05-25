function Test262Error(message) {
  this.name = "Test262Error";
  this.message = message ?? "";
}

function assertSameValue(actual, expected, label) {
  if (actual !== expected) {
    throw new Test262Error(label + ": expected " + expected + " got " + actual);
  }
}

function assertThrowsTypeError(func, label) {
  try {
    func();
  } catch (error) {
    console.log(label, "threw", error.name);
    if (error.name !== "TypeError") {
      throw new Test262Error(label + ": expected TypeError");
    }
    return;
  }
  throw new Test262Error(label + ": no throw");
}

let classStringExpression = `(
class {
  static #m = 'test262';

  static access() {
    return this.#m;
  }
}
)`;

let evalClass = function (_eval) {
  return _eval(classStringExpression);
};

console.log("before C1");
let C1 = evalClass(eval);
console.log("before C2");
let C2 = evalClass(eval);

console.log("before C1 own");
assertSameValue(C1.access(), "test262", "C1 own");
console.log("before C2 own");
assertSameValue(C2.access(), "test262", "C2 own");
console.log("before C1 on C2");
assertThrowsTypeError(function() {
  C1.access.call(C2);
}, "C1 on C2");
console.log("before C2 on C1");
assertThrowsTypeError(function() {
  C2.access.call(C1);
}, "C2 on C1");
console.log("done");
