class C {
  func = () => {
    super.prop = "test262";
  };

  static staticFunc = () => {
    super.staticProp = "static test262";
  };
}

let c = new C();
c.func();
console.log("prop", c.prop);
if (c.prop !== "test262") {
  console.log("bad instance");
  throw new Error("bad instance");
}

C.staticFunc();
console.log("static", C.staticProp);
if (C.staticProp !== "static test262") {
  console.log("bad static");
  throw new Error("bad static");
}
