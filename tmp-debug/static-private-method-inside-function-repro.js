class C {
  static #f() { return 42; }
  g() { return this.#f(); }
}

console.log("call", new C().g.call(C));

function h() {
  try {
    console.log("inner", new C().g());
  } catch (error) {
    console.log("inner threw", error.name);
  }
}

h();
