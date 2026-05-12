var value = [];
var iterationResult, iter, x;
iter = (function*() {
  var counter = 0;
  for ([x = yield] of [[]]) {
    counter += 1;
  }
  assert.sameValue(counter, 1);
})();
iterationResult = iter.next();
assert.sameValue(iterationResult.value, undefined);
assert.sameValue(iterationResult.done, false);
assert.sameValue(x, undefined);
iterationResult = iter.next(86);
assert.sameValue(iterationResult.value, undefined);
assert.sameValue(iterationResult.done, true);
assert.sameValue(x, 86);
