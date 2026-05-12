var nextCount = 0;
var returnCount = 0;
var thisMatches = false;
var argsPresent = false;
var argsLength = -1;
var _;
var iterable = {};
var iterator = {
  next: function() {
    nextCount += 1;
    return { done: nextCount > 10 };
  },
  return: function() {
    returnCount += 1;
    thisMatches = this === iterator;
    argsPresent = !!arguments;
    argsLength = arguments.length;
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
  console.log(thisMatches);
  console.log(argsPresent);
  console.log(argsLength);
  counter += 1;
}

console.log(counter);
