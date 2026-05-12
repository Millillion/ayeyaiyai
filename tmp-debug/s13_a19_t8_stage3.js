var a = 1, b = "a";
var __obj = { a: 2 };

with (__obj) {
  while (1) {
    var __func = function() {
      return a;
    };
    break;
  }
}

delete __obj;

if (__func() !== 2) {
  throw 1;
}

var __obj = { a: 3, b: "b" };

with (__obj) {
  var __func = function() {
    return b;
  };
}

delete __obj;

if (__func() !== "b") {
  throw 2;
}

with ({ a: 99, b: "c" }) {
  if (__func() !== "b") {
    throw 3;
  }
}
