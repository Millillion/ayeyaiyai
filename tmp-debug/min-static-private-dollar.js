class C {
  static #$;
  static $(value) {
    C.#$ = value;
    return C.#$;
  }
}
C.$(1);
