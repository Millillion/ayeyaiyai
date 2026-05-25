var o = {
  __proto__: function () {},
};

console.log(Object.getPrototypeOf(o).name);
console.log(Object.getPrototypeOf(o).name === "__proto__");
console.log(Object.getPrototypeOf(o) === Function.prototype);
