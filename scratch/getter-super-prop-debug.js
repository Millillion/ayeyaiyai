var proto = {
  _x: 42,
  get x() {
    return 'proto' + this._x;
  }
};

var object = {
  get x() {
    return super.x;
  }
};

Object.setPrototypeOf(object, proto);

console.log(object.x);
console.log(typeof object.x);
console.log(object.x === 'proto42');
console.log(String(object.x));
console.log(object._x);
console.log(typeof object._x);
console.log(object._x === 42);
console.log(Object.getPrototypeOf(object)._x);
console.log(typeof Object.getPrototypeOf(object)._x);
console.log(Object.getPrototypeOf(object)._x === 42);
