class C {
  set #m(v) { this._v = v; }

  method(v) { this.#m = v; }

  B = class {
    method(o, v) {
      o.#m = v;
    }

    get m() { return this.#m; }

    #m;
  }
}

let c = new C();
let innerB = new c.B();

console.log("before inner");
innerB.method(innerB, "test262");
console.log("after inner", innerB.m);

console.log("before outer");
c.method("outer class");
console.log("after outer", c._v);

try {
  console.log("before invalid");
  innerB.method(c, "foo");
  console.log("after invalid");
} catch (e) {
  console.log("caught", e.name);
}
