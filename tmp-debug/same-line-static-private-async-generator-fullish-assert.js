function Test262Error(message) {
  this.name = "Test262Error";
  this.message = message || "";
}
function assert(mustBeTrue, message) {
  if (mustBeTrue === true) return;
  throw new Test262Error(message);
}
assert.sameValue = function(actual, expected, message) {
  if (actual === expected) return;
  throw new Test262Error(message);
};
function verifyProperty(obj, name, desc, options) {
  var actual = Object.getOwnPropertyDescriptor(obj, name);
  assert.sameValue(actual.enumerable, desc.enumerable);
  assert.sameValue(actual.configurable, desc.configurable);
  assert.sameValue(actual.writable, desc.writable);
}

class C {
  async *m() { return 42; } static async * #$(value) {
    yield * await value;
  }
  static async * #_(value) {
    yield * await value;
  }
  static async * #o(value) {
    yield * await value;
  }
  static async * #℘(value) {
    yield * await value;
  }
  static async * #ZW_‌_NJ(value) {
    yield * await value;
  }
  static async * #ZW_‍_J(value) {
    yield * await value;
  };
  static get $() { return this.#$; }
  static get _() { return this.#_; }
  static get o() { return this.#o; }
  static get ℘() { return this.#℘; }
  static get ZW_‌_NJ() { return this.#ZW_‌_NJ; }
  static get ZW_‍_J() { return this.#ZW_‍_J; }
}

var c = new C();
assert(
  !Object.prototype.hasOwnProperty.call(c, "m"),
  "m doesn't appear as an own property on the C instance"
);
assert.sameValue(c.m, C.prototype.m);
verifyProperty(C.prototype, "m", {
  enumerable: false,
  configurable: true,
  writable: true,
}, {restore: true});

c.m().next().then(function(v) {
  assert.sameValue(v.value, 42);
  assert.sameValue(v.done, true);
  function assertions() {
    function $DONE(error) {
      if (error) {
        throw new Test262Error('Test262:AsyncTestFailure')
      }
      console.log("inner done");
    }
    Promise.all([
      C.$([1]).next(),
      C._([1]).next(),
      C.o([1]).next(),
      C.℘([1]).next(),
      C.ZW_‌_NJ([1]).next(),
      C.ZW_‍_J([1]).next(),
    ]).then(results => {
      assert.sameValue(results[0].value, 1);
      assert.sameValue(results[1].value, 1);
      assert.sameValue(results[2].value, 1);
      assert.sameValue(results[3].value, 1);
      assert.sameValue(results[4].value, 1);
      assert.sameValue(results[5].value, 1);
    }).then($DONE, $DONE);
  }
  return Promise.resolve(assertions());
}).then(function() {
  console.log("outer done");
}, function(error) {
  throw error;
});
