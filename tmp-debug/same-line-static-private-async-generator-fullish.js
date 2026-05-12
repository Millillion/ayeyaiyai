class C {
  async *m() { return 42; } static async * #$(value) {
    yield * await value;
  }
  static async * #_(value) {
    yield * await value;
  }
  static async * #o(value) {
    yield * await value;
  }
  static get $() { return this.#$; }
  static get _() { return this.#_; }
  static get o() { return this.#o; }
}

var c = new C();
c.m().next().then(function(v) {
  console.log(v.value);
  console.log(v.done);
  function assertions() {
    function $DONE(error) {
      if (error) {
        console.log("inner error");
      }
      console.log("inner done");
    }
    Promise.all([
      C.$([1]).next(),
      C._([1]).next(),
      C.o([1]).next(),
    ]).then(results => {
      console.log(results[0].value);
      console.log(results[1].value);
      console.log(results[2].value);
    }).then($DONE, $DONE);
  }
  return Promise.resolve(assertions());
}).then(function() {
  console.log("outer done");
}, function(error) {
  console.log("outer error", error);
});
