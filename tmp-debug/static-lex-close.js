var test262 = 'outer scope';
var probe;

class C {
  static {
    let test262 = 'first block';
    console.log('first', test262);
  }
  static {
    console.log('before probe', test262);
    probe = test262;
  }
}

console.log('after', probe, test262);
