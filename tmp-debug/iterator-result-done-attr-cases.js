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

function run(label, result) {
  firstIterResult = result;
  i = 0;
  for (var x of iterable) {
    i++;
  }
  console.log(label);
  console.log(i);
}

run(1, { value: null, done: undefined });
run(2, { value: null });
run(3, { value: null, done: null });
run(4, { value: null, done: false });
run(5, { value: null, done: true });
run(6, { value: null, done: 1 });
run(7, { value: null, done: 0 });
run(8, { value: null, done: -0 });
run(9, { value: null, done: NaN });
run(10, { value: null, done: '' });
run(11, { value: null, done: '0' });
run(12, { value: null, done: Symbol() });
run(13, { value: null, done: {} });

firstIterResult = { value: null };
Object.defineProperty(firstIterResult, 'done', {
  get: function() {
    return true;
  }
});
i = 0;
for (var x of iterable) {
  i++;
}
console.log(14);
console.log(i);
