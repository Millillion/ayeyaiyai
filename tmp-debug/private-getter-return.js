class C {
  #x;

  get #g() {
    return this.#x;
  }

  m(value) {
    this.#x = value;
    return this.#g;
  }
}

const c = new C();
const result = c.m(1);
console.log(typeof result);
console.log(result === 1);
console.log(result + 1);
console.log(result);
