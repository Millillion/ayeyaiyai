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
for (var x of iterable) {
  console.log(x);
  i++;
  guard++;
  if (guard >= 4) {
    break;
  }
}
console.log(i);
