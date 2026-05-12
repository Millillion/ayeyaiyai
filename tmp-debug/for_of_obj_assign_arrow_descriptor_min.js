function Test262Error(message) {
  this.name = "Test262Error";
  this.message = message || "";
}

function assertSameValue(actual, expected, message) {
  if (actual !== expected) {
    throw new Test262Error(message || (String(actual) + " !== " + String(expected)));
  }
}

function assert(condition, message) {
  if (condition !== true) {
    throw new Test262Error(message || "assertion failed");
  }
}

var __getOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;
var __hasOwnProperty = Function.prototype.call.bind(Object.prototype.hasOwnProperty);
var __propertyIsEnumerable = Function.prototype.call.bind(Object.prototype.propertyIsEnumerable);

function verifyNameDescriptor(obj) {
  var originalDesc = __getOwnPropertyDescriptor(obj, "name");
  assert(__hasOwnProperty(obj, "name"), "own name");
  assertSameValue(originalDesc.value, "arrow", "value");
  assertSameValue(obj.name, "arrow", "read");
  assertSameValue(originalDesc.enumerable, false, "enumerable desc");
  assertSameValue(__propertyIsEnumerable(obj, "name"), false, "enumerable call");
  assertSameValue(originalDesc.writable, false, "writable desc");
  assertSameValue(originalDesc.configurable, true, "configurable desc");
}

var arrow;
var counter = 0;

for ({ arrow = () => {} } of [{}]) {
  verifyNameDescriptor(arrow);
  counter += 1;
}

assertSameValue(counter, 1, "counter");
