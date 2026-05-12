class C {
  static set #f(v) {
    this._v = v;
  }

  static access() {
    const arrowFunction = () => {
      this.#f = "Test262";
    };

    arrowFunction();
  }
}

try {
  C.access();
  console.log("direct", C._v);
} catch (e) {
  console.log("direct threw", e.name);
}

try {
  C.access.call({});
  console.log("call ok");
} catch (e) {
  console.log("call threw", e.name);
}
