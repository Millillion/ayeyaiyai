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

firstIterResult = /regexp/;
var count = 0;
for (var x of iterable) {
  count++;
  console.log(100 + count);
  if (count > 3) break;
}
console.log(200 + count);
