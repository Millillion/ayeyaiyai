var iter;
iter = (function*() {
  for ([x = yield] of [[]]) {}
})();
iter.next();
