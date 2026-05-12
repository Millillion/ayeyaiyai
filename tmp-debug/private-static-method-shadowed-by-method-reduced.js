class C {
  static #m() { return 'outer class'; }

  static methodAccess() {
    return this.#m();
  }

  static B = class {
    #m() { return 'inner class'; }

    static access(o) {
      return o.#m();
    }
  }
}

console.log(C.methodAccess());
console.log('before new');
let b = new C.B();
console.log('after new');
console.log('before inner access');
console.log(C.B.access(b));
console.log('after inner access');
try {
  console.log(C.B.access(C));
  console.log('no throw');
} catch (e) {
  console.log('caught');
  console.log(e.name);
}
