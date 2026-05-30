var seen = 0;

var C = class {
  constructor() {
    seen = seen + 1;
  }
};

var c = new C();

if (seen !== 0) {
  throw new Error("not zero");
}

