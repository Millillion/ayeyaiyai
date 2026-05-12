
function Test262Error(message) {
  this.name = "Test262Error";
  this.message = message ?? "";
}

function __formatIdentityFreeValue(value) {
  switch (value === null ? "null" : typeof value) {
    case "string":
      return typeof JSON !== "undefined" ? JSON.stringify(value) : "\"" + value + "\"";
    case "bigint":
      return String(value) + "n";
    case "number":
      if (value === 0 && 1 / value === -Infinity) {
        return "-0";
      }
      return String(value);
    case "boolean":
    case "undefined":
    case "null":
      return String(value);
  }
}

function __sameValue(left, right) {
  if (left === right) {
    return left !== 0 || 1 / left === 1 / right;
  }
  return left !== left && right !== right;
}

function __assertToString(value) {
  var basic = __formatIdentityFreeValue(value);
  if (basic) {
    return basic;
  }
  try {
    return String(value);
  } catch (error) {
    if (error && error.name === "TypeError") {
      return Object.prototype.toString.call(value);
    }
    throw error;
  }
}

function assert(mustBeTrue, message) {
  if (mustBeTrue === true) {
    return;
  }
  if (message === undefined) {
    message = "Expected true but got " + __assertToString(mustBeTrue);
  }
  throw new Test262Error(message);
}

globalThis.assert = assert;

function __assert(condition, message) {
  assert(condition, message);
}

function __assertSameValue(actual, expected, message) {
  try {
    if (__sameValue(actual, expected)) {
      return;
    }
  } catch (error) {
    throw new Test262Error((message ?? "") + " (_isSameValue operation threw) " + error);
  }

  if (message === undefined) {
    message = "";
  } else {
    message += " ";
  }

  message += "Expected SameValue(«" + __assertToString(actual) + "», «" + __assertToString(expected) + "») to be true";
  throw new Test262Error(message);
}

function __assertNotSameValue(actual, expected, message) {
  if (!__sameValue(actual, expected)) {
    return;
  }

  if (message === undefined) {
    message = "";
  } else {
    message += " ";
  }

  message += "Expected SameValue(«" + __assertToString(actual) + "», «" + __assertToString(expected) + "») to be false";
  throw new Test262Error(message);
}

function __ayyAssertThrows(expectedErrorConstructor, func, message) {
  var expectedName, actualName;

  if (typeof func !== "function") {
    throw new Test262Error("assert.throws requires two arguments: the error constructor and a function to run");
  }

  if (message === undefined) {
    message = "";
  } else {
    message += " ";
  }

  try {
    func();
  } catch (thrown) {
    if (typeof thrown !== "object" || thrown === null) {
      throw new Test262Error(message + "Thrown value was not an object!");
    } else if (thrown.constructor !== expectedErrorConstructor) {
      expectedName = expectedErrorConstructor.name;
      actualName = thrown.constructor.name;
      if (expectedName === actualName) {
        message += "Expected a " + expectedName + " but got a different error constructor with the same name";
      } else {
        message += "Expected a " + expectedName + " but got a " + actualName;
      }
      throw new Test262Error(message);
    }
    return;
  }

  throw new Test262Error(message + "Expected a " + expectedErrorConstructor.name + " to be thrown but no exception was thrown at all");
}

assert._isSameValue = __sameValue;
assert._toString = __assertToString;
assert.sameValue = __assertSameValue;
assert.notSameValue = __assertNotSameValue;
assert.throws = __ayyAssertThrows;

function compareArray(actual, expected) {
  if (actual.length !== expected.length) {
    return false;
  }
  for (var i = 0; i < actual.length; i += 1) {
    if (!__sameValue(actual[i], expected[i])) {
      return false;
    }
  }
  return true;
}

compareArray.format = function (arrayLike) {
  return "" + arrayLike;
};

function __ayyAssertCompareArray() {}

assert.compareArray = __ayyAssertCompareArray;


function $DONE(error) {
  if (error !== undefined) {
    throw error;
  }
}

function verifyProperty(obj, name, desc, options) {
  var actual = Object.getOwnPropertyDescriptor(obj, name);
  __assertSameValue(actual.enumerable, desc.enumerable);
  __assertSameValue(actual.configurable, desc.configurable);
  __assertSameValue(actual.writable, desc.writable);
}




class C {
  async *m() { return 42; } static async * #$(value) {
    yield * await value;
  }
  static async * #_(value) {
    yield * await value;
  }
  static async * #o(value) {
    yield * await value;
  }
  static async * #℘(value) {
    yield * await value;
  }
  static async * #ZW_‌_NJ(value) {
    yield * await value;
  }
  static async * #ZW_‍_J(value) {
    yield * await value;
  };
  static get $() {
   return this.#$;
  }
  static get _() {
   return this.#_;
  }
  static get o() {
   return this.#o;
  }
  static get ℘() { // DO NOT CHANGE THE NAME OF THIS FIELD
   return this.#℘;
  }
  static get ZW_‌_NJ() { // DO NOT CHANGE THE NAME OF THIS FIELD
   return this.#ZW_‌_NJ;
  }
  static get ZW_‍_J() { // DO NOT CHANGE THE NAME OF THIS FIELD
   return this.#ZW_‍_J;
  }

}

var c = new C();

__assert(
  !Object.prototype.hasOwnProperty.call(c, "m"),
  "m doesn't appear as an own property on the C instance"
);
__assertSameValue(c.m, C.prototype.m);

verifyProperty(C.prototype, "m", {
  enumerable: false,
  configurable: true,
  writable: true,
}, {restore: true});

c.m().next().then(function(v) {
  __assertSameValue(v.value, 42);
  __assertSameValue(v.done, true);

  function assertions() {
    // Cover $DONE handler for async cases.
    function $DONE(error) {
      if (error) {
        throw new Test262Error('Test262:AsyncTestFailure')
      }
    }
    Promise.all([
      C.$([1]).next(),
      C._([1]).next(),
      C.o([1]).next(),
      C.℘([1]).next(), // DO NOT CHANGE THE NAME OF THIS FIELD
      C.ZW_‌_NJ([1]).next(), // DO NOT CHANGE THE NAME OF THIS FIELD
      C.ZW_‍_J([1]).next(), // DO NOT CHANGE THE NAME OF THIS FIELD
    ]).then(results => {

      __assertSameValue(results[0].value, 1);
      __assertSameValue(results[1].value, 1);
      __assertSameValue(results[2].value, 1);
      __assertSameValue(results[3].value, 1);
      __assertSameValue(results[4].value, 1);
      __assertSameValue(results[5].value, 1);

    }).then($DONE, $DONE);
  }

  return Promise.resolve(assertions());
}).then($DONE, $DONE);
