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
var result = c.$(1);
console.log(typeof result);
console.log(result === 1);
console.log(result);
