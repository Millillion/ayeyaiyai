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
  static async * #℘(value) {
    yield * await value;
  }
  static async * #ZW_‌_NJ(value) {
    yield * await value;
  }
  static async * #ZW_‍_J(value) {
    yield * await value;
  };
  static get $() { return this.#$; }
  static get _() { return this.#_; }
  static get o() { return this.#o; }
  static get ℘() { return this.#℘; }
  static get ZW_‌_NJ() { return this.#ZW_‌_NJ; }
  static get ZW_‍_J() { return this.#ZW_‍_J; }
}

var c = new C();
c.m().next().then(function(v) {
  console.log(v.value);
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
      C.℘([1]).next(),
      C.ZW_‌_NJ([1]).next(),
      C.ZW_‍_J([1]).next(),
    ]).then(results => {
      console.log(results[0].value);
      console.log(results[1].value);
      console.log(results[2].value);
      console.log(results[3].value);
      console.log(results[4].value);
      console.log(results[5].value);
    }).then($DONE, $DONE);
  }
  return Promise.resolve(assertions());
}).then(function() {
  console.log("outer done");
}, function(error) {
  console.log("outer error", error);
});
