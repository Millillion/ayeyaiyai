function Test262Error(message) {
  this.name = "Test262Error";
  this.message = message || "";
}

var nextCount = 0;
var returnCount = 0;
var iterable = {};
var iterator = {
  next: function() {
    nextCount += 1;
    if (nextCount === 2) {
      throw new Test262Error("second next");
    }
    return { done: false };
  },
  return: function() {
    returnCount += 1;
  }
};
iterable[Symbol.iterator] = function() {
  return iterator;
};

var step = { value: iterable };
var x;
var caught = 0;
try {
  [x,,] = step.value;
} catch (thrown) {
  caught = 1;
}

console.log(caught);
console.log(nextCount);
console.log(returnCount);
