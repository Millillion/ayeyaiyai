function Test262Error(message) {
  this.name = "Test262Error";
  this.message = message || "";
}

function assertSameValue(actual, expected, message) {
  if (actual !== expected) {
    throw new Test262Error(message || (String(actual) + " !== " + String(expected)));
  }
}

var arrow;
var counter = 0;

for ({ arrow = () => {} } of [{}]) {
  assertSameValue(arrow.name, "arrow", "arrow default name");
  counter += 1;
}

assertSameValue(counter, 1, "counter");
