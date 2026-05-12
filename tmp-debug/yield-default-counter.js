var iterationResult, iter, x;
iter = (function*() {
  var counter = 0;
  for ([x = yield] of [[]]) {
    counter += 1;
  }
  if (counter !== 1) {
    throw 1;
  }
})();
iterationResult = iter.next();
iterationResult = iter.next(86);
if (iterationResult.done !== true) {
  throw 2;
}
