
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


function __propertyHelperHasOwn(obj, name) {
  return Object.getOwnPropertyDescriptor(obj, name) !== undefined;
}

function __propertyHelperUnsupported(name) {
  throw new Test262Error("unsupported propertyHelper fallback: " + name);
}

function verifyProperty(obj, name, desc, options) {
  return __propertyHelperUnsupported("verifyProperty");
}

function verifyEqualTo(obj, name, value) {
  return __propertyHelperUnsupported("verifyEqualTo");
}

function verifyWritable(obj, name, verifyProp, value) {
  return __propertyHelperUnsupported("verifyWritable");
}

function verifyNotWritable(obj, name, verifyProp, value) {
  return __propertyHelperUnsupported("verifyNotWritable");
}

function verifyEnumerable(obj, name) {
  return __propertyHelperUnsupported("verifyEnumerable");
}

function verifyNotEnumerable(obj, name) {
  return __propertyHelperUnsupported("verifyNotEnumerable");
}

function verifyConfigurable(obj, name) {
  return __propertyHelperUnsupported("verifyConfigurable");
}

function verifyNotConfigurable(obj, name) {
  return __propertyHelperUnsupported("verifyNotConfigurable");
}

function verifyCallableProperty(obj, name, functionName, length, desc) {
  return __propertyHelperUnsupported("verifyCallableProperty");
}

function verifyPrimordialProperty(obj, name, desc) {
  return __propertyHelperUnsupported("verifyPrimordialProperty");
}

function verifyPrimordialCallableProperty(obj, name, functionName, length, desc) {
  return __propertyHelperUnsupported("verifyPrimordialCallableProperty");
}

// This file was procedurally generated from the following sources:
// - src/dstr-binding/obj-ptrn-rest-skip-non-enumerable.case
// - src/dstr-binding/default/func-expr-dflt.template
var o = {a: 3, b: 4};
Object.defineProperty(o, "x", { value: 4, enumerable: false });

var callCount = 0;
var f;
f = function({...rest} = o) {
  __assertSameValue(rest.x, undefined);

  verifyProperty(rest, "a", {
    enumerable: true,
    writable: true,
    configurable: true,
    value: 3
  });

  verifyProperty(rest, "b", {
    enumerable: true,
    writable: true,
    configurable: true,
    value: 4
  });
  callCount = callCount + 1;
};

f();
__assertSameValue(callCount, 1, 'function invoked exactly once');

