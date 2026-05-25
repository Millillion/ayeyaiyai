class C {
  #field;

  static *isNameIn() {
    return #field in (yield);
  }
}

let iter1 = C.isNameIn();
iter1.next();
console.log(iter1.next(new C()).value);

let iter2 = C.isNameIn();
iter2.next();
console.log(iter2.next({}).value);
