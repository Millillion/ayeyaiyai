var iterable = {};
var i, firstIterResult;

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
i = 0;
for (var x of iterable) { i++; }
console.log(i);

firstIterResult = { value: null };
i = 0;
for (var x of iterable) { i++; }
console.log(i);

firstIterResult = { value: null, done: null };
i = 0;
for (var x of iterable) { i++; }
console.log(i);

firstIterResult = { value: null, done: false };
i = 0;
for (var x of iterable) { i++; }
console.log(i);
