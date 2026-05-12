class C {
  static async #$(value) {
    return await value;
  }
  static async #_(value) {
    return await value;
  }
  static async $(value) {
    return await this.#$(value);
  }
  static async _(value) {
    return await this.#_(value);
  }
}

function $DONE(error) {
  if (error) {
    throw error;
  }
}

Promise.all([
  C.$(1),
  C._(2),
]).then(results => {
  console.log(results[0]);
  console.log(results[1]);
  if (results[0] !== 1) {
    throw new Error("bad first");
  }
  if (results[1] !== 2) {
    throw new Error("bad second");
  }
}).then($DONE, $DONE);
