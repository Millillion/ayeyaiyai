var ctorPromise;

function checkAs(actual, expected, name) {
  if (actual === expected) {
    return;
  }
  if (name === 'eval') throw new EvalError();
  if (name === 'range') throw new RangeError();
  if (name === 'reference') throw new ReferenceError();
  if (name === 'syntax') throw new SyntaxError();
  throw new TypeError();
}

function hasProp(obj, name, expected) {
  checkAs(Object.prototype.hasOwnProperty.call(obj, name), expected, 'eval');
  checkAs(Reflect.has(obj, name), expected, 'eval');
}

var C = class {
  async * #m() {
    return 42;
  }

  get ref() {
    return this.#m;
  }

  constructor() {
    hasProp(this, '#m', false);
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

hasProp(C.prototype, '#m', false);
hasProp(C, '#m', false);
hasProp(c, '#m', false);

checkAs(c.ref, other.ref, 'type');
checkAs(c.ref.name, '#m', 'syntax');
