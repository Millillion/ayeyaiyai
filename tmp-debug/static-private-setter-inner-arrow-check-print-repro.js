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
console.log("before compare", C._v);
if (C._v !== "Test262") {
  console.log("bad direct branch");
  throw new Error("bad direct");
}

let threw = false;
try {
  C.access.call({});
} catch (e) {
  console.log("caught", e.name);
  threw = e instanceof TypeError;
}
console.log("threw", threw);
if (!threw) {
  throw new Error("missing throw");
}
