class C {
  #$_;
  set #$(value) {
    this.#$_ = value;
  }
  $(value) {
    this.#$ = value;
    return this.#$_;
  }
}

var c = new C();
if (c.$(1) !== 1) {
  throw new TypeError();
}
