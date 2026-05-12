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
var iter = iterable[Symbol.iterator]();
var step = iter.next();
console.log(step.done);
console.log(step.done ? 1 : 0);
step = iter.next();
console.log(step.done);
console.log(step.done ? 1 : 0);
