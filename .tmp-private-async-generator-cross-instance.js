var C = class {
  async * #m() {
    return 42;
  }

  get ref() {
    return this.#m;
  }
};

var c = new C();
var other = new C();

if (c.ref !== other.ref) {
  throw new TypeError();
}

if (c.ref.name !== '#m') {
  throw new SyntaxError();
}
