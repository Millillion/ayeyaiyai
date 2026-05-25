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

function assertSameValue(actual, expected, label) {
  if (actual != expected) {
    throw new Error(label + ': expected ' + expected + ' got ' + actual);
  }
}

function assertThrowsTypeError(func, label) {
  try {
    func();
  } catch (error) {
    print(label, 'threw', error.name);
    if (error.name != 'TypeError') {
      throw new Error(label + ': expected TypeError');
    }
    return;
  }
  throw new Error(label + ': no throw');
}

print('before C1');
let C1 = evalClass(eval1);
print('before C2');
let C2 = evalClass(eval2);

print('before C1 own');
assertSameValue(C1.access(), 'test262', 'C1 own');
print('before C2 own');
assertSameValue(C2.access(), 'test262', 'C2 own');

print('before C1 on C2');
assertThrowsTypeError(function() {
  C1.access.call(C2);
}, 'C1 on C2');

print('before C2 on C1');
assertThrowsTypeError(function() {
  C2.access.call(C1);
}, 'C2 on C1');

print('done');
