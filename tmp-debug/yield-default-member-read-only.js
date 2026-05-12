var iterationResult, iter, value;
iter = (function*() {
  for ([x = yield] of [[]]) {}
})();
iterationResult = iter.next();
value = iterationResult.value;
