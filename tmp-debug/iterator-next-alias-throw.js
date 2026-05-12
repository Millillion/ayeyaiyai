function Test262Error(message) {
  this.name = "Test262Error";
  this.message = message || "";
}

var nextCount = 0;
var iterator = {
  next: function() {
    nextCount += 1;
    if (nextCount === 2) {
      throw new Test262Error("second next");
    }
    return { done: false };
  }
};

var alias = iterator;
var caught = 0;
try {
  alias.next();
  alias.next();
} catch (thrown) {
  caught = 1;
}

console.log(caught);
console.log(nextCount);
