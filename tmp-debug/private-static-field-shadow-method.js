class C {
  static #m = () => 'outer class';

  static fieldAccess() {
    return this.#m();
  }

  static B = class {
    #m() { return 'inner class'; }

    static access(o) {
      return o.#m();
    }
  }
}

let b = new C.B();
console.log(C.fieldAccess());
console.log(C.B.access(b));
try {
  console.log(C.B.access(C));
} catch (e) {
  console.log(e.name);
}
