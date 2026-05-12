var iterationResult, iter, x;
iter = (function*() {
  for ([x = yield] of [[]]) {}
})();
iterationResult = iter.next();
if (iterationResult.done !== false) {
  throw 1;
}
