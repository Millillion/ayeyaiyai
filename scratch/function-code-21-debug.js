"use strict";

function f() {
    return this;
}

console.log((new f()) === this, typeof (new f()));
