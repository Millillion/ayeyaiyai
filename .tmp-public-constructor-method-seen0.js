var seen = 0;

var C = class {
  m() {
    seen = seen + 1;
  }

  constructor() {
    this.m();
  }
};

var c = new C();

if (seen !== 0) {
  throw new Error("method did run");
}
