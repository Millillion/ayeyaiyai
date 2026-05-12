class C {
  set #m(v) { this._v = v; }
  v = (this.#m = 53, this._v);
}

let c = new C();
console.log(c._v, c.v);
