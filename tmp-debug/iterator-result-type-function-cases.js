var iterable = {};
var firstIterResult;

iterable[Symbol.iterator] = function() {
  var finalIterResult = { value: null, done: true };
  var nextIterResult = firstIterResult;

  return {
    next: function() {
      var iterResult = nextIterResult;
      nextIterResult = finalIterResult;
      return iterResult;
    }
  };
};

function expectThrow(marker) {
  try {
    for (var x of iterable) {}
    console.log(marker * 10 + 1);
  } catch (e) {
    console.log(marker * 10 + 2);
  }
}

firstIterResult = true;
expectThrow(1);
firstIterResult = false;
expectThrow(2);
firstIterResult = "string";
expectThrow(3);
firstIterResult = undefined;
expectThrow(4);
firstIterResult = null;
expectThrow(5);
firstIterResult = 4;
expectThrow(6);
firstIterResult = NaN;
expectThrow(7);
firstIterResult = Symbol("s");
expectThrow(8);
