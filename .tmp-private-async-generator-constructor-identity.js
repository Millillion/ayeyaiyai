var C = class {
  async * #m() { return 42; }

  get ref() { return this.#m; }

  constructor() {
    if (typeof this.#m !== 'function') {
      throw new EvalError();
    }
    if (this.ref !== this.#m) {
      throw new RangeError();
    }
    if (this.#m !== (() => this)().#m) {
      throw new ReferenceError();
    }
    if (this.#m.name !== '#m') {
      throw new SyntaxError();
    }
  }
};

var c = new C();
