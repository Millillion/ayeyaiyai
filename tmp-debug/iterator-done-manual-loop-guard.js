var iterable = {};
var firstIterResult;

iterable[Symbol.iterator] = function() {
  var finalIterResult = { value: null, done: true };
  var nextIterResult = firstIterResult;
  return {
    next: function() {
      var iterResult = nextIterResult;
      nextIterResult = finalIterResult;
      return iterResult;
    }
  };
};

firstIterResult = { value: null, done: undefined };
var i = 0;
var guard = 0;
var iter = iterable[Symbol.iterator]();
while (guard < 4) {
  var step = iter.next();
  console.log(step.done);
  if (step.done) {
    break;
  }
  var x = step.value;
  i++;
  guard++;
}
console.log(i);
