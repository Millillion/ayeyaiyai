function checkEval(actual, expected) {
  if (actual !== expected) {
    throw new EvalError();
  }
}

function checkRange(actual, expected) {
  if (actual !== expected) {
    throw new RangeError();
  }
}

function checkRef(actual, expected) {
  if (actual !== expected) {
    throw new ReferenceError();
  }
}

function checkSyntax(actual, expected) {
  if (actual !== expected) {
    throw new SyntaxError();
  }
}

var C = class {
  async * #m() { return 42; }

  get ref() { return this.#m; }

  constructor() {
    checkEval(typeof this.#m, 'function');
    checkRange(this.ref, this.#m);
    checkRef(this.#m, (() => this)().#m);
    checkSyntax(this.#m.name, '#m');
  }
};

var c = new C();
