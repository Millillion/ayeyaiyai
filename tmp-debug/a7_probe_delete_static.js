var o = { aa: 1, ba: 2, ca: 3 };
delete o["ba"];
console.log(o["aa"]);
console.log(o["ba"]);
console.log(o["ca"]);
