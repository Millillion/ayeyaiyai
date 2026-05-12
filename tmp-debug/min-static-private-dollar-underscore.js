class C {
  async *m() { return 42; }
  static #$;
  static #_;
  static $(value) {
    C.#$ = value;
    return C.#$;
  }
  static _(value) {
    C.#_ = value;
    return C.#_;
  }
}

var c = new C();
c.m().next().then(function(v) {
  if (v.value !== 42) throw new Error("value");
  if (v.done !== true) throw new Error("done");
  function assertions() {
    if (C.$(1) !== 1) throw new Error("dollar");
    if (C._(1) !== 1) throw new Error("underscore");
  }
  return Promise.resolve(assertions());
}).then(function() {}, function(error) { throw error; });
