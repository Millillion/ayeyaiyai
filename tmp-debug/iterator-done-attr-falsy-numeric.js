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

firstIterResult = { value: null, done: 0 };
i = 0;
for (var x of iterable) { i++; }
console.log(i);

firstIterResult = { value: null, done: -0 };
i = 0;
for (var x of iterable) { i++; }
console.log(i);

firstIterResult = { value: null, done: NaN };
i = 0;
for (var x of iterable) { i++; }
console.log(i);

firstIterResult = { value: null, done: '' };
i = 0;
for (var x of iterable) { i++; }
console.log(i);
