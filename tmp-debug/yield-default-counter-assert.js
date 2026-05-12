var iterationResult, iter, x;
iter = (function*() {
  var counter = 0;
  for ([x = yield] of [[]]) {
    counter += 1;
  }
  assert.sameValue(counter, 1);
})();
iterationResult = iter.next();
iterationResult = iter.next(86);
assert.sameValue(iterationResult.done, true);
assert.sameValue(x, 86);
