function Test262Error() {}

var returnCount = 0;
var unreachable = 0;
var iterable = {};
var iterator = {
  return: function() {
    returnCount += 1;
    throw new Test262Error();
  }
};
var iter;
iterable[Symbol.iterator] = function() { return iterator; };

function* g() {
  var counter = 0;
  for ([ {}[ yield ] ] of [iterable]) {
    unreachable += 1;
    counter += 1;
  }
  console.log("after loop", counter);
}

iter = g();
console.log("before next");
iter.next();
console.log("after next");
try {
  iter.return();
  console.log("after return no throw");
} catch (e) {
  console.log("caught", returnCount, unreachable);
}
console.log("final", returnCount, unreachable);
