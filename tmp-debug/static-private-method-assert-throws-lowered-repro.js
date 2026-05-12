class C {
  static #f() { return 42; }
  g() { return this.#f(); }
}

console.log("first", new C().g.call(C));

let __ayy_assert_throws_callback_1 = function() {
  new C().g();
};
let __ayy_assert_throws_caught_1 = false;
try {
  __ayy_assert_throws_callback_1();
} catch {
  console.log("in catch");
  __ayy_assert_throws_caught_1 = true;
}
console.log("caught", __ayy_assert_throws_caught_1);
if (__ayy_assert_throws_caught_1 === false) {
  throw undefined;
}
