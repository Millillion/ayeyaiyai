let global1 = $262.createRealm().global;
let global2 = $262.createRealm().global;
let eval1 = global1.eval;
let eval2 = global2.eval;

let classStringExpression = `(
class {
  static #m = 'test262';

  static access() {
    return this.#m;
  }
}
)`;

let evalClass = function (_eval) {
  return _eval(classStringExpression);
};

let C1 = evalClass(eval1);
let C2 = evalClass(eval2);

if (C1.access() != 'test262') {
  throw new Error('C1 own value');
}
throw new Error('after C1 own');
