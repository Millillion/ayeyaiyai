function Test262Error(message) {
  this.message = message || "";
}

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

firstIterResult = /regexp/;
console.log(10);
for (var x of iterable) {
  console.log(11);
}
console.log(12);

firstIterResult = {};
console.log(20);
for (var x of iterable) {
  console.log(21);
}
console.log(22);

firstIterResult = new Proxy({}, {
  get: function(receiver, name) {
    if (name === 'done') {
      return true;
    }
    if (name === 'value') {
      return null;
    }
    throw new Test262Error('unreachable true proxy');
  }
});
console.log(30);
for (var x of iterable) {
  console.log(31);
}
console.log(32);

firstIterResult = new Proxy({}, {
  get: function(receiver, name) {
    if (name === 'done') {
      return false;
    }
    if (name === 'value') {
      return 23;
    }
    throw new Test262Error('unreachable false proxy');
  }
});
var i = 0;
console.log(40);
for (var x of iterable) {
  console.log(x);
  i++;
}
console.log(i);
console.log(42);
