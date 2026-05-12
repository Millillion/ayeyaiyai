var nextCount = 0;
var returnCount = 0;
var _;
var iterable = {};
var iterator = {
  next: function() {
    nextCount += 1;
    return { done: nextCount > 10 };
  },
  return: function() {
    returnCount += 1;
    throw new Test262Error();
  }
};
iterable[Symbol.iterator] = function() {
  return iterator;
};

var counter = 0;

assert.throws(Test262Error, function() {
  for ([ _ ] of [iterable]) {
    counter += 1;
  }
  counter += 1;
});

assert.sameValue(counter, 0);
assert.sameValue(nextCount, 1);
assert.sameValue(returnCount, 1);
