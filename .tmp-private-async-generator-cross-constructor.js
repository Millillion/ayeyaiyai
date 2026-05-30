function checkAs(actual, expected, name) {
  if (actual === expected) {
    return;
  }
  if (name === 'range') throw new RangeError();
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
    checkAs(this.#m.name, '#m', 'syntax');
  }
};

var c = new C();
var other = new C();

checkAs(c.ref, other.ref, 'type');
checkAs(c.ref.name, '#m', 'syntax');
