var iterationResult, iter, x;
iter = (function*() {
  for ([x = yield] of [[]]) {}
})();
iterationResult = iter.next();
iterationResult = iter.next(86);
if (x !== 86) {
  throw 1;
}
