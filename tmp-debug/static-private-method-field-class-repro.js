class C {
  static #xVal;
  static #x(value) {
    C.#xVal = value;
    return C.#xVal;
  }
  static x() {
    return C.#x(42);
  }
}

console.log(C.x());
