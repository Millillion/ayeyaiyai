class C {
  async *m() { return 42; } static async * #$(value) {
    yield * await value;
  }
  static async * #_(value) {
    yield * await value;
  }
  static async * #o(value) {
    yield * await value;
  }
  static async * #℘(value) {
    yield * await value;
  }
  static async * #ZW_‌_NJ(value) {
    yield * await value;
  }
  static async * #ZW_‍_J(value) {
    yield * await value;
  };
  static get $() {
   return this.#$;
  }
  static get _() {
   return this.#_;
  }
  static get o() {
   return this.#o;
  }
  static get ℘() { // DO NOT CHANGE THE NAME OF THIS FIELD
   return this.#℘;
  }
  static get ZW_‌_NJ() { // DO NOT CHANGE THE NAME OF THIS FIELD
   return this.#ZW_‌_NJ;
  }
  static get ZW_‍_J() { // DO NOT CHANGE THE NAME OF THIS FIELD
   return this.#ZW_‍_J;
  }

}

var c = new C();

__assert(
  !Object.prototype.hasOwnProperty.call(c, "m"),
  "m doesn't appear as an own property on the C instance"
);
__assertSameValue(c.m, C.prototype.m);

verifyProperty(C.prototype, "m", {
  enumerable: false,
  configurable: true,
  writable: true,
}, {restore: true});

console.log(Object.prototype.hasOwnProperty.call(c, "m"));
console.log(c.m === C.prototype.m);
