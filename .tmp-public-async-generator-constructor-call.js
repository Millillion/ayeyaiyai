var seen = 0;

var C = class {
  async * m() {
    return 42;
  }

  constructor() {
    this.m().next().then(function(result) {
      seen = seen + 1;
    });
  }
};

var c = new C();

if (seen !== 1) {
  throw new Error("constructor callback did not run");
}

