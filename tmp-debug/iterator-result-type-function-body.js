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

function run() {
  var count = 0;
  try {
    for (var x of iterable) {
      console.log(count);
      count++;
      if (count > 3) break;
    }
    console.log(100 + count);
  } catch (e) {
    console.log(200 + count);
  }
}

firstIterResult = true;
run();
