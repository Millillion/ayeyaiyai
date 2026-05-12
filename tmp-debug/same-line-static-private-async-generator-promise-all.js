class C {
  async *m() { return 42; } static async * #$(value) {
    yield * await value;
  }
  static get $() {
    return this.#$;
  }
}

Promise.all([
  C.$([1]).next(),
]).then(function(results) {
  console.log(results[0].value);
  console.log(results[0].done);
});
