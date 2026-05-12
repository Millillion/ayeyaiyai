var iter, x;
iter = (function*() { for ([x = yield] of [[]]) {} })();
iter.next();
console.log(
  x,
  x === undefined,
  x === 0,
  x === 1,
  x === -1,
  x === -1073741823,
  x === -1073741818,
  undefined === -1073741823,
  undefined === -1073741818
);
