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
print(seen);
