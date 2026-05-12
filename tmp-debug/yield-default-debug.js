var iterationResult, iter, x;
iter = (function*() {
  for ([x = yield] of [[]]) {}
})();
iterationResult = iter.next();
console.log("first", iterationResult.value, iterationResult.done, x);
iterationResult = iter.next(86);
console.log("second", iterationResult.value, iterationResult.done, x);
