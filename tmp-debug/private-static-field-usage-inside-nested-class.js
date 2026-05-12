class C {
  static #m = 'outer class';

  static B = class {
    static fieldAccess(o) {
      return o.#m;
    }
  }
}

try {
  console.log(C.B.fieldAccess(C));
} catch (e) {
  console.log("field throw");
  console.log(e.name);
}

try {
  C.B.methodAccess(C.B);
  console.log("no throw");
} catch (e) {
  console.log("method throw");
  console.log(e.name);
}
