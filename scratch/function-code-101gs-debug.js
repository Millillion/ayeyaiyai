var x = 3;

function f() {
    x = this;
    return "a";
}

var result = (function() {
    "use strict";
    return "ab".replace("b", f);
}());

console.log(result, x === this, x, this);
