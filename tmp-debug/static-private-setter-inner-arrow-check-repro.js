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

C.access();
if (C._v !== "Test262") {
  throw new Error("bad direct");
}

let threw = false;
try {
  C.access.call({});
} catch (e) {
  threw = e instanceof TypeError;
}
if (!threw) {
  throw new Error("missing throw");
}
