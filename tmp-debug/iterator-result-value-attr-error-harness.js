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
  var expectedName, actualName;
  try {
    func();
  } catch (thrown) {
    if (typeof thrown !== "object" || thrown === null) {
      throw new Test262Error((message || "") + "Thrown value was not an object!");
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
  throw new Test262Error((message || "") + "Expected throw");
}

var iterable = {};
var iterationCount = 0;
var returnCount = 0;

iterable[Symbol.iterator] = function() {
  return {
    next: function() {
      return {
        done: false,
        get value() {
          throw new Test262Error();
        }
      };
    },
    return: function() {
      returnCount += 1;
      return {};
    }
  };
};

__ayyAssertThrows(Test262Error, function() {
  for (var x of iterable) {
    iterationCount += 1;
  }
});

console.log(iterationCount);
console.log(returnCount);
__assertSameValue(iterationCount, 0, "The loop body is not evaluated");
__assertSameValue(returnCount, 0, "Iterator is not closed.");
