function Test262Error(message) {
  this.message = message || "";
}

function __assertSameValue(actual, expected) {
  if (actual !== expected) {
    throw new Test262Error("Expected " + expected + " but got " + actual);
  }
}

function testWithTypedArrayConstructors(f) {
  var ctors = [Uint8Array];
  for (var i = 0; i < ctors.length; ++i) {
    f(ctors[i]);
  }
}

testWithTypedArrayConstructors(function(Constructor) {
  class Typed extends Constructor {}

  var arr = new Typed(2);

  __assertSameValue(arr.length, 2);
});
