var iterationResult, iter, x;
iter = (function*() {
  for ([x = yield] of [[]]) {}
})();
iterationResult = iter.next();
assert.sameValue(iterationResult.done, false);
