
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



// Copyright (C) 2018 André Bargull. All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.

var AsyncFunction = async function(){}.constructor;

__ayyAssertThrows(SyntaxError, function() {
    AsyncFunction("import.meta");
}, "import.meta in AsyncFunctionBody");

__ayyAssertThrows(SyntaxError, function() {
    AsyncFunction("a = import.meta", "");
}, "import.meta in FormalParameters");

