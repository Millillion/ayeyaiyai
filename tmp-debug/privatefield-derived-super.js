class Outer {
  #x = 42;

  constructor() {
    console.log("outer ctor body");
  }

  innerclass() {
    console.log("innerclass start");
    return class extends Outer {
      constructor() {
        console.log("inner ctor before super");
        super();
        console.log("inner ctor after super");
      }

      f() {
        this.#x = 1;
      }
    };
  }

  value() {
    return this.#x;
  }
}

console.log("before outer");
var outer = new Outer();
console.log("after outer");
var Inner = outer.innerclass();
console.log("after innerclass");
var i = new Inner();
console.log("after inner");
console.log(outer.value());
console.log(i.value());
i.f();
console.log(outer.value());
console.log(i.value());
