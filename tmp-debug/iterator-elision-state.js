function Test262Error(message) {
  this.name = "Test262Error";
  this.message = message || "";
}

var nextCount = 0;
var returnCount = 0;
var iterable = {};
var x;
var iterator = {
  next: function() {
    nextCount += 1;
    if (nextCount === 2) {
      throw new Test262Error("second next");
    }
    return { done: nextCount > 10 };
  },
  return: function() {
    returnCount += 1;
  }
};
iterable[Symbol.iterator] = function() {
  return iterator;
};

var counter = 0;
try {
  for ([ x , , ] of [iterable]) {
    counter += 1;
  }
  counter += 1;
} catch (thrown) {
  counter = 100 + nextCount;
}

if (counter !== 102) {
  throw new Test262Error("counter=" + counter + " nextCount=" + nextCount + " returnCount=" + returnCount);
}
if (returnCount !== 0) {
  throw new Test262Error("returnCount=" + returnCount);
}
