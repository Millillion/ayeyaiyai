var seen = 0;

var C = class {
  async * #m() {
    return 42;
  }

  get ref() {
    return this.#m;
  }

  constructor() {
    this.#m().next().then(function(result) {
      if (result.value !== 42) {
        throw new Error("bad value");
      }
      if (result.done !== true) {
        throw new Error("bad done");
      }
      seen = seen + 1;
    });
  }
};

var c = new C();
c.ref().next().then(function(result) {
  if (result.value !== 42) {
    throw new Error("bad external value");
  }
  if (result.done !== true) {
    throw new Error("bad external done");
  }
  seen = seen + 1;
});

if (seen !== 2) {
  throw new Error("callbacks did not run");
}
