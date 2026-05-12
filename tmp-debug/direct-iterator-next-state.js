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

try {
  iterator.next();
  iterator.next();
} catch (thrown) {
}

if (nextCount !== 2) {
  throw new Test262Error("nextCount=" + nextCount);
}
