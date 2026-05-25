function Test262Error(message) {
  this.name = "Test262Error";
  this.message = message === undefined ? "" : message;
}

function __ayyAssertThrows(expectedErrorConstructor, func, message) {
  if (typeof func !== "function") {
    throw new Test262Error("assert.throws requires two arguments: the error constructor and a function to run");
  }

  try {
    func();
  } catch (thrown) {
    if (typeof thrown !== "object" || thrown === null) {
      throw new Test262Error("Thrown value was not an object!");
    } else if (thrown.constructor !== expectedErrorConstructor) {
      throw new Test262Error("Unexpected error constructor");
    }
    return;
  }

  throw new Test262Error("Expected an exception");
}

var C = class {
  f = this.g();
}

class D extends C {
  g() { this.#m; }
  get #m() { return 42; }
}

__ayyAssertThrows(TypeError, function() {
  var d = new D();
});
