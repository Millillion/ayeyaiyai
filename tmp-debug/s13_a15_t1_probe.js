var __obj = new __FACTORY();
console.log(typeof obj);
console.log(__obj.prop);
console.log(__obj.slot.prop);

function __FACTORY() {
    this.prop = 1;
    var obj = {};
    obj.prop = "A";
    obj.slot = this;
    return obj;
}
