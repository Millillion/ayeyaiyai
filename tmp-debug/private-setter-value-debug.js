class C {
  #x;
  set #s(value) {
    this.#x = value;
  }
  m(value) {
    this.#s = value;
    return this.#x;
  }
}

var c = new C();
var result = c.m(1);
console.log(typeof result);
console.log(result === 1);
console.log(result);
