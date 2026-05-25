function f() { "use strict"; return this; }
console.log(typeof f.bind(this)(), typeof this, f.bind(this)() !== this);
