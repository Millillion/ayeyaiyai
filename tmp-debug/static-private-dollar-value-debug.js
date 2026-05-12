class C {
  static #$;
  static $(value) {
    C.#$ = value;
    return C.#$;
  }
}

var result = C.$(1);
console.log(typeof result);
console.log(result === 1);
console.log(result);
