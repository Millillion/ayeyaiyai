class C {
  static #f() { return 42; }
  static g() {
    const arrowFunction = () => this.#f();
    return arrowFunction();
  }
}

try {
  console.log("direct", C.g());
} catch (error) {
  console.log("direct threw", error.name);
}

try {
  console.log("call", C.g.call({}));
} catch (error) {
  console.log("call threw", error.name);
}
