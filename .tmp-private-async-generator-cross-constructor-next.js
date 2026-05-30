var ctorPromise;

function checkAs(actual, expected, name) {
  if (actual === expected) {
    return;
  }
  if (name === 'range') throw new RangeError();
  if (name === 'reference') throw new ReferenceError();
  if (name === 'syntax') throw new SyntaxError();
  throw new TypeError();
}

var C = class {
  async * #m() {
    return 42;
  }

  get ref() {
    return this.#m;
  }

  constructor() {
    checkAs(typeof this.#m, 'function', 'range');
    checkAs(this.ref, this.#m, 'range');
    checkAs(this.#m, (() => this)().#m, 'range');

    var ctorIter = this.#m();
    var p = ctorIter.next();
    ctorPromise = p.then(({ value, done }) => {
      checkAs(value, 42, 'reference');
      checkAs(done, true, 'reference');
    }, function(error) { throw error; });

    checkAs(this.#m.name, '#m', 'syntax');
  }
};

var c = new C();
var other = new C();

checkAs(c.ref, other.ref, 'type');
checkAs(c.ref.name, '#m', 'syntax');
