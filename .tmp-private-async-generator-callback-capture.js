function checkAs(actual, expected, name) {
  if (actual === expected) {
    return;
  }
  if (name === 'reference') throw new ReferenceError();
  throw new TypeError();
}

var C = class {
  async * #m() {
    return 42;
  }

  get ref() {
    return this.#m;
  }
};

var c = new C();
var p = c.ref().next();

p.then(() => {
  var iter = c.ref();
  return iter.next().then(({ value, done }) => {
    checkAs(value, 42, 'reference');
    checkAs(done, true, 'reference');
  });
}).then(function() {}, function(error) { throw error; });
