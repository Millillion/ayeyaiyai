var count = 0;

class A {
  constructor() {
    console.log("A constructor before", count);
    count++;
    console.log("A constructor after", count);
  }
  increment() {
    console.log("A increment before", count);
    count++;
    console.log("A increment after", count);
  }
}

class B extends A {
  constructor() {
    console.log("B before super");
    super();
    console.log("B after super");
    (_ => {
      console.log("arrow before super.increment");
      return super.increment();
    })();
    console.log("B after arrow");
  }
}

var bar = new B();
console.log("final", count);
