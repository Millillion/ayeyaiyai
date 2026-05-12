this.__obj = { p1: 1 };
console.log(typeof __obj);
console.log(__obj.p1 === 1);
__obj.p1 = "w1";
console.log(this.__obj.p1 === "w1");
