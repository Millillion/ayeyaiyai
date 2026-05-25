function __sameValue(left, right) {
  if (left === right) {
    return left !== 0 || 1 / left === 1 / right;
  }
  return left !== left && right !== right;
}

function f() {
  return false;
}

var x = f();
console.log("x", x);
console.log("same", __sameValue(x, false));
