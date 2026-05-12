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
