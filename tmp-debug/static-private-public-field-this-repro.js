class C {
  static #xVal;
  static x(value) {
    this.#xVal = value;
    return this.#xVal;
  }
}

console.log(C.x(42));
