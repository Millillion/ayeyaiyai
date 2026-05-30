var seen = 0;

var C = class {
  constructor() {
    seen = seen + 1;
  }
};

var c = new C();

if (seen !== 2) {
  throw new Error("not two");
}

