var iterable = {};
var firstIterResult;

iterable[Symbol.iterator] = function() {
  var finalIterResult = { value: null, done: true };
  var nextIterResult = firstIterResult;
  return {
    next: function() {
      console.log(99);
      var iterResult = nextIterResult;
      nextIterResult = finalIterResult;
      return iterResult;
    }
  };
};

firstIterResult = { value: null, done: undefined };
var guard = 0;
var iter = iterable[Symbol.iterator]();
while (guard < 2) {
  var step = iter.next();
  console.log(step.done);
  guard++;
}
