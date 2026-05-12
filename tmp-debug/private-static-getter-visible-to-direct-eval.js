class C {
  static get #m() {
    return 'Test262';
  }

  static getWithEval() {
    return eval('this.#m');
  }
}

class D {
  static get #m() {
    throw new Error('should never be executed');
  }
}

try {
  console.log(C.getWithEval());
} catch (e) {
  console.log("first throw");
  console.log(e.name);
}

try {
  console.log(C.getWithEval.call(D));
  console.log("no throw");
} catch (e) {
  console.log("second throw");
  console.log(e.name);
}
