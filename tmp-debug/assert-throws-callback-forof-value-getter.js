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

try {
  (function() {
    for (var x of iterable) {
      iterationCount += 1;
    }
  })();
  console.log(1);
} catch (e) {
  console.log(2);
}
console.log(iterationCount);
console.log(returnCount);
