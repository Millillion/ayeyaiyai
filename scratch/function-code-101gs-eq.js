var x = 3; function f(){ x = this; return "a"; } (function(){ "use strict"; return "ab".replace("b", f); }()); console.log(x === this, x != this, x !== this, x == this);
