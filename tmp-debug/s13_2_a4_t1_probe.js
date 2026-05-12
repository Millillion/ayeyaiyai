function __func() {}

console.log(typeof __func.prototype);
console.log(__func.prototype.constructor === __func);

var __constructor_was__enumed;
for (__prop in __func.prototype) {
  if (__prop === "constructor") {
    __constructor_was__enumed = true;
  }
}
console.log(__constructor_was__enumed);
