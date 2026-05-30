function __assertSameValue(actual, expected) {
  if (actual !== expected) {
    throw "fail";
  }
}

var other = $262.createRealm().global;
var C = new other.Function();
C.prototype = null;

class B extends function() {} {
  constructor() {
    super();
  }
}

var b = Reflect.construct(B, [], C);
__assertSameValue(Object.getPrototypeOf(b), other.Object.prototype);
