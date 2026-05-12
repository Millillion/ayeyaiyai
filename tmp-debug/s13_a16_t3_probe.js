var __obj = new function __func(arg) {
    this.prop = arg;
    return { feat: ++arg };
}(5);

console.log(__obj.prop);
console.log(__obj.feat);
console.log(__obj.feat === 6);
console.log(typeof __func);
