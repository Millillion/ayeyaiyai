function checkAs(actual, expected) {
  if (actual !== expected) {
    throw new RangeError();
  }
}

var C = class {
  async * #m() { return 42; }

  get ref() { return this.#m; }

  constructor() {
    checkAs(typeof this.#m, 'function');
    checkAs(this.ref, this.#m);
    checkAs(this.#m, (() => this)().#m);
    checkAs(this.#m.name, '#m');
  }
};

var c = new C();
