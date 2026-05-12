function __assertSameValue(actual, expected) {
  if (actual !== expected) throw new Error("sameValue");
}
function assert(condition) {
  if (!condition) throw new Error("assert");
}
assert.sameValue = __assertSameValue;

class C {
  async *m() { return 42; }
  static #$;
  static $(value) {
    C.#$ = value;
    return C.#$;
  }
}

var c = new C();
c.m().next().then(function(v) {
  assert.sameValue(v.value, 42);
  assert.sameValue(v.done, true);
  function assertions() {
    assert.sameValue(C.$(1), 1);
  }
  return Promise.resolve(assertions());
}).then(function() {}, function(error) { throw error; });
