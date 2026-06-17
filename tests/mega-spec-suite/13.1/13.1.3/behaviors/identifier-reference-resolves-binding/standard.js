// behavior: identifier-reference-resolves-binding
// expected: pass
// goal: script
// size: standard
// variant: script.sloppy

var referencedValue = 43;
function __ayyRun() {
var score = 0;
score = score + referencedValue;
if (referencedValue !== 43) { throw "reference"; }
var v0 = 0;
score = score + v0;
if (v0 !== 0) { throw "v0"; }
if ((v0 + 1) !== 1) { throw "branch0"; }
var v1 = 1;
score = score + v1;
if (v1 !== 1) { throw "v1"; }
var v2 = 2;
score = score + v2;
if (v2 !== 2) { throw "v2"; }
var v3 = 3;
score = score + v3;
if (v3 !== 3) { throw "v3"; }
var v4 = 4;
score = score + v4;
if (v4 !== 4) { throw "v4"; }
var v5 = 5;
score = score + v5;
if (v5 !== 5) { throw "v5"; }
var v6 = 6;
score = score + v6;
if (v6 !== 6) { throw "v6"; }
var v7 = 7;
score = score + v7;
if (v7 !== 7) { throw "v7"; }
var v8 = 8;
score = score + v8;
if (v8 !== 8) { throw "v8"; }
var v9 = 9;
score = score + v9;
if (v9 !== 9) { throw "v9"; }
var v10 = 10;
score = score + v10;
if (v10 !== 10) { throw "v10"; }
var v11 = 11;
score = score + v11;
if (v11 !== 11) { throw "v11"; }
var v12 = 12;
score = score + v12;
if (v12 !== 12) { throw "v12"; }
var v13 = 13;
score = score + v13;
if (v13 !== 13) { throw "v13"; }
var v14 = 14;
score = score + v14;
if (v14 !== 14) { throw "v14"; }
var v15 = 15;
score = score + v15;
if (v15 !== 15) { throw "v15"; }
var v16 = 16;
score = score + v16;
if (v16 !== 16) { throw "v16"; }
var v17 = 17;
score = score + v17;
if (v17 !== 17) { throw "v17"; }
if ((v17 + 1) !== 18) { throw "branch17"; }
var v18 = 18;
score = score + v18;
if (v18 !== 18) { throw "v18"; }
var v19 = 19;
score = score + v19;
if (v19 !== 19) { throw "v19"; }
var v20 = 20;
score = score + v20;
if (v20 !== 20) { throw "v20"; }
var v21 = 21;
score = score + v21;
if (v21 !== 21) { throw "v21"; }
var v22 = 22;
score = score + v22;
if (v22 !== 22) { throw "v22"; }
var v23 = 23;
score = score + v23;
if (v23 !== 23) { throw "v23"; }
var v24 = 24;
score = score + v24;
if (v24 !== 24) { throw "v24"; }
var v25 = 25;
score = score + v25;
if (v25 !== 25) { throw "v25"; }
var v26 = 26;
score = score + v26;
if (v26 !== 26) { throw "v26"; }
var v27 = 27;
score = score + v27;
if (v27 !== 27) { throw "v27"; }
var v28 = 28;
score = score + v28;
if (v28 !== 28) { throw "v28"; }
var v29 = 29;
score = score + v29;
if (v29 !== 29) { throw "v29"; }
return score;
}
console.log("ok", __ayyRun());
