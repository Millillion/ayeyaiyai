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
    return {};
  }
};
iterable[Symbol.iterator] = function() {
  return iterator;
};

for ([ _ ] of [iterable]) {
  console.log(returnCount);
}
