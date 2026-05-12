class A {
  constructor(arg) {
    return arg;
  }
}

class C extends A {
  #x;

  constructor(arg) {
    super(arg);
  }
}

console.log(1);
var containsprivatex = new C();
console.log(2);
try {
  new C(containsprivatex);
  console.log(3);
} catch (error) {
  console.log(4);
}
console.log(5);
