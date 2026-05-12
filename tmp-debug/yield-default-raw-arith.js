var iter, x;
iter = (function*() { for ([x = yield] of [[]]) {} })();
iter.next();
console.log(x + 0, x | 0, x - 0);
console.log(undefined + 0, undefined | 0, undefined - 0);
