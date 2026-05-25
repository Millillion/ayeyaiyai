let C1 = (
class {
  static #m = 'test262';

  static access() {
    return this.#m;
  }
}
);

console.log("c1", C1.access());
