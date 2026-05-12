class C {
  static #m = 'outer class';

  static fieldAccess() {
    return this.#m;
  }

  static B = class {
    set #m(v) { this._v = v; }

    static access(o) {
      o.#m = 'inner class';
    }
  }
}

let b = new C.B();
console.log(C.fieldAccess());
C.B.access(b);
console.log(b._v);
try {
  C.B.access(C);
  console.log('no throw');
} catch (e) {
  console.log(e.name);
}
