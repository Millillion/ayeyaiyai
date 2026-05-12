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
var iter = iterable[Symbol.iterator]();
var step = iter.next();
if (!step.done) {
  i++;
}
step = iter.next();
if (!step.done) {
  i++;
}
console.log(i);
