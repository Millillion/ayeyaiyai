class C {
  static #f() { return 42; }
  g() {
    return this.#f();
  }
}

try {
  console.log("call", new C().g.call(C));
} catch (error) {
  console.log("call threw", error.name);
}

try {
  console.log("direct", new C().g());
} catch (error) {
  console.log("direct threw", error.name);
}
