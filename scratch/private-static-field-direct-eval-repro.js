let classStringExpression = `(
class {
  static #m = 'test262';

  static access() {
    return this.#m;
  }
}
)`;

let evalClass = function () {
  return eval(classStringExpression);
};

let C1 = evalClass();
let C2 = evalClass();

console.log("c1", C1.access());
console.log("c2", C2.access());

try {
  C1.access.call(C2);
  console.log("bad1");
} catch (error) {
  console.log("throw1", error.name);
}

try {
  C2.access.call(C1);
  console.log("bad2");
} catch (error) {
  console.log("throw2", error.name);
}
