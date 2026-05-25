class Class {
  #field;

  static isNameIn(value) {
    return #field in value;
  }
}

console.log(Class.isNameIn({}));
console.log(Class.isNameIn(new Class()));
