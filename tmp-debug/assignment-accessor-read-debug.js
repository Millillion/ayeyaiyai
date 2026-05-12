function __sameValue(left, right) {
  if (left === right) {
    return left !== 0 || 1 / left === 1 / right;
  }
  return left !== left && right !== right;
}

let x = 0;

let C = class {
  get [x = 1]() {
    return 2;
  }

  set [x = 1](v) {
    return 2;
  }

  static get [x = 1]() {
    return 2;
  }

  static set [x = 1](v) {
    return 2;
  }
};

let c = new C();
console.log("before");
let value = c[x = 1];
console.log("after");
console.log(value);
console.log(x);
