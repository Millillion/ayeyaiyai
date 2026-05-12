var A = class B {
  method() {}
  static method() {}
}

if (typeof A !== "function") {
  throw new EvalError();
}
if (typeof A.prototype.method !== "function") {
  throw new RangeError();
}
if (typeof A.method !== "function") {
  throw new ReferenceError();
}
if (typeof B !== "undefined") {
  throw new SyntaxError();
}
