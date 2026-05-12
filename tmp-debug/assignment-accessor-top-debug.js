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

if (c[x = 1] !== 2) throw new Error("1");
console.log("p1");
if ((c[x = 1] = 2) !== 2) throw new Error("2");
console.log("p2");
if (C[x = 1] !== 2) throw new Error("3");
console.log("p3");
if ((C[x = 1] = 2) !== 2) throw new Error("4");
console.log("p4");
if (c[String(x = 1)] !== 2) throw new Error("5");
console.log("p5");
if ((c[String(x = 1)] = 2) !== 2) throw new Error("6");
console.log("p6");
if (C[String(x = 1)] !== 2) throw new Error("7");
console.log("p7");
if ((C[String(x = 1)] = 2) !== 2) throw new Error("8");
console.log("p8");
if (x !== 1) throw new Error("9");
console.log("p9");
