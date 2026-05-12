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

firstIterResult = /regexp/;
for (var x of iterable) {}
console.log(90);

firstIterResult = {};
for (var x of iterable) {}
console.log(100);

firstIterResult = new Proxy({}, {
  get: function(receiver, name) {
    if (name === "done") {
      return true;
    }
    if (name === "value") {
      return null;
    }
    throw 999;
  }
});
for (var x of iterable) {
  throw 1000;
}
console.log(110);

firstIterResult = new Proxy({}, {
  get: function(receiver, name) {
    if (name === "done") {
      return false;
    }
    if (name === "value") {
      return 23;
    }
    throw 999;
  }
});
var i = 0;
for (var x of iterable) {
  console.log(x);
  i++;
}
console.log(i);
