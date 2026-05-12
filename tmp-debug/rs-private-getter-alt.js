class C {
  #$_;
  #__;
  #o_;
  #℘_;
  #ZW_NJ_;
  #ZW_J_;

  get #$() {
    return this.#$_;
  }
  get #_() {
    return this.#__;
  }
  get #o() {
    return this.#o_;
  }
  get #℘() {
    return this.#℘_;
  }
  get #ZW_NJ() {
    return this.#ZW_NJ_;
  }
  get #ZW_J() {
    return this.#ZW_J_;
  }

  $(value) {
    this.#$_ = value;
    return this.#$;
  }
  _(value) {
    this.#__ = value;
    return this.#_;
  }
  o(value) {
    this.#o_ = value;
    return this.#o;
  }
  ℘(value) {
    this.#℘_ = value;
    return this.#℘;
  }
  ZW_NJ(value) {
    this.#ZW_NJ_ = value;
    return this.#ZW_NJ;
  }
  ZW_J(value) {
    this.#ZW_J_ = value;
    return this.#ZW_J;
  }
}

const c = new C();
console.log("start");
console.log("$", c.$(1));
console.log("_", c._(2));
console.log("o", c.o(3));
console.log("℘", c.℘(4));
console.log("ZW_NJ", c.ZW_NJ(5));
console.log("ZW_J", c.ZW_J(6));
