var obj;

var __obj = new __FACTORY();

console.log(obj.prop);
console.log(__obj.prop);
console.log(__obj.slot.prop);

function __FACTORY() {
    this.prop = 1;
    obj = {};
    obj.prop = "A";
    obj.slot = this;
    return obj;
}
