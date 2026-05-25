let Child;

class Parent {
  #field;

  static init() {
    Child = class {
      #field;

      static isNameIn(value) {
        return #field in value;
      }
    };
  }
}

Parent.init();

console.log("parent", Child.isNameIn(new Parent()));
console.log("child", Child.isNameIn(new Child()));
