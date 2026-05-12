class C {
  async *m() { return 42; } static async * #$(value) {
    yield * await value;
  }
  static get $() {
    return this.#$;
  }
}

var c = new C();
c.m().next().then(function(v) {
  console.log(v.value);
  console.log(v.done);
  return Promise.resolve(C.$([1]).next().then(function(result) {
    console.log(result.value);
    console.log(result.done);
  }));
}).then(function() {
  console.log("done");
}, function(error) {
  console.log("error", error);
});
