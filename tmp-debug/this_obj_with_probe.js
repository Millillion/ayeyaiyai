this.__obj = { p1: 1 };
with (__obj) {
    p1 = "w1";
}
console.log(__obj.p1 === "w1");
console.log(this.__obj.p1 === "w1");
