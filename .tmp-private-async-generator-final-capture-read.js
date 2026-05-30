var ctorPromise;

var C = class {
  async * #m() {
    return 42;
  }

  get ref() {
    return this.#m;
  }

  constructor() {
    ctorPromise = this.#m().next().then(function() {});
  }
};

var c = new C();

ctorPromise.then(() => {
  var f = c.ref;
  if (f !== c.ref) {
    throw new URIError();
  }
}).then(function() {}, function(error) { throw error; });
