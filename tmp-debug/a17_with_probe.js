__obj = { p1: 1, getRight: function () { return "right"; } };
with (__obj) {
  p1 = "w1";
  getRight = function () { return false; };
  console.log(p1 === "w1");
  console.log(getRight() === false);
}
console.log(__obj.p1 === "w1");
console.log(__obj.getRight() === false);
