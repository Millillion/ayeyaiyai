class C {
  async *m() { return 42; } static async * #$(value) {
    yield * await value;
  }
  static get $() {
    return this.#$;
  }
}

C.$([1]).next().then(function(result) {
  console.log(result.value);
  console.log(result.done);
});
