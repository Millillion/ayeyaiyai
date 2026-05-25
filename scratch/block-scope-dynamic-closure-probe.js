function fn(one) {
  var x = one + 1;
  let y = one + 2;
  const u = one + 4;
  {
    let z = one + 3;
    const v = one + 5;
    function f() {
      assert.sameValue(one, 1);
      assert.sameValue(x, 2);
    }

    f();
  }
}
fn(1);
