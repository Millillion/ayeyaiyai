var seen = 0;

var C = class {
  async * m() {
    return 42;
  }
};

var c = new C();
c.m().next().then(function(result) {
  if (result.done !== true) {
    throw new Error("bad done");
  }
  seen = seen + 1;
});

if (seen !== 1) {
  throw new Error("callback did not run");
}

