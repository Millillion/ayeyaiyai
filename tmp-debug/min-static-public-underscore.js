class C {
  static #$;
  static $(value) {
    C.#$ = value;
    return C.#$;
  }
  static _(value) {
    return value;
  }
}
C.$(1);
C._(1);
