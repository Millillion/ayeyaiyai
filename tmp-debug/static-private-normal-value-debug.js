class C {
  static #x;
  static m(value) {
    C.#x = value;
    return C.#x;
  }
}

var result = C.m(1);
console.log(typeof result);
console.log(result === 1);
console.log(result);
