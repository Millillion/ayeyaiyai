var nextCount = 0;
var returnCount = 0;
var thisValue = null;
var args = null;
var _;
var iterable = {};
var iterator = {
  next: function() {
    nextCount += 1;
    return { done: nextCount > 10 };
  },
  return: function() {
    returnCount += 1;
    thisValue = this;
    args = arguments;
    return {};
  }
};
iterable[Symbol.iterator] = function() {
  return iterator;
};

var counter = 0;
for ([ _ ] of [iterable]) {
  console.log(nextCount);
  console.log(returnCount);
  console.log(thisValue === iterator);
  console.log(!!args);
  console.log(args.length);
  counter += 1;
}
console.log(counter);
