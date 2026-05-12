var iterationResult, iter, x;
iter = (function*() {
  for ([x = yield] of [[]]) {}
})();
iterationResult = iter.next();
if (x !== undefined) {
  throw 1;
}
