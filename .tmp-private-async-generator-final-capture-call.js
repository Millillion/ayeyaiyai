var ctorPromise;

function check(actual, expected) {
  if (actual !== expected) {
    throw new ReferenceError();
  }
}

var C = class {
  async * #m() {
    return 42;
  }

  get ref() {
    return this.#m;
  }

  constructor() {
    ctorPromise = this.#m().next().then(function() {});
  }
};

var c = new C();

ctorPromise.then(() => {
  var iter = c.ref();
  return iter.next().then(({ value, done }) => {
    check(value, 42);
    check(done, true);
  });
}).then(function() {}, function(error) { throw error; });
