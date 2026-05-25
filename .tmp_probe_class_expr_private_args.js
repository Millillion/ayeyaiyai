var callCount = 0;
var C = class {
  #method() {
    console.log("inside", arguments.length, arguments[0], arguments[1], arguments[2]);
    callCount = callCount + 1;
  }

  get method() {
    return this.#method;
  }
};

new C().method(42, null,);
console.log("after", callCount);
