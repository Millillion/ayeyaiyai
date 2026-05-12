var nextCount = 0;
var returnCount = 0;
var unreachable = 0;
var thisValue = null;
var args = null;
var iterator = {
  next: function() {
    nextCount += 1;
    return { done: false, value: undefined };
  },
  return: function() {
    returnCount += 1;
    return {};
  }
};
var iterable = {};
iterable[Symbol.iterator] = function() {
  return iterator;
};
function* g() {
  var result;
  var vals = iterable;
  result = [ {} = yield ] = vals;
  unreachable += 1;
}
var iter = g();
console.log('a');
console.log(iter.next().done);
console.log('b', nextCount, returnCount);
var result = iter.return(777);
console.log('c', nextCount, returnCount, unreachable);
console.log(result.done, result.value, thisValue === iterator, args && args.length);
