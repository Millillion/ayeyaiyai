class C {
  static #xVal;
  static #x(value) {
    console.log("inside");
    this.#xVal = value;
    return this.#xVal;
  }
  static x() {
    return this.#x(42);
  }
}

console.log(C.x());
