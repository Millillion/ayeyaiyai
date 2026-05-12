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

function check(label, actual, expected) {
  console.log(label);
  console.log(actual);
  if (actual !== expected) {
    throw new Error(label);
  }
}

check("1", c[x = 1], 2);
check("2", c[x = 1] = 2, 2);
check("3", C[x = 1], 2);
check("4", C[x = 1] = 2, 2);
check("5", c[String(x = 1)], 2);
check("6", c[String(x = 1)] = 2, 2);
check("7", C[String(x = 1)], 2);
check("8", C[String(x = 1)] = 2, 2);
check("9", x, 1);
