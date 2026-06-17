// behavior: private-identifier-prefix
// expected: pass
// goal: script
// size: standard
// variant: script.strict

class __AyyPrivateBox { #value; constructor(value) { this.#value = value; } getValue() { return this.#value; } setValue(value) { this.#value = value; } }
function __ayyRun() {
var privateBox = new __AyyPrivateBox(31);
if (privateBox.getValue() !== 31) { throw "private-get"; }
privateBox.setValue(37);
if (privateBox.getValue() !== 37) { throw "private-set"; }
var v0 = 0;
if (v0 !== 0) { throw "v0"; }
if ((v0 + 1) !== 1) { throw "branch0"; }
var v1 = 1;
if (v1 !== 1) { throw "v1"; }
var v2 = 2;
if (v2 !== 2) { throw "v2"; }
var v3 = 3;
if (v3 !== 3) { throw "v3"; }
var v4 = 4;
if (v4 !== 4) { throw "v4"; }
var v5 = 5;
if (v5 !== 5) { throw "v5"; }
var v6 = 6;
if (v6 !== 6) { throw "v6"; }
var v7 = 7;
if (v7 !== 7) { throw "v7"; }
var v8 = 8;
if (v8 !== 8) { throw "v8"; }
var v9 = 9;
if (v9 !== 9) { throw "v9"; }
var v10 = 10;
if (v10 !== 10) { throw "v10"; }
var v11 = 11;
if (v11 !== 11) { throw "v11"; }
var v12 = 12;
if (v12 !== 12) { throw "v12"; }
var v13 = 13;
if (v13 !== 13) { throw "v13"; }
var v14 = 14;
if (v14 !== 14) { throw "v14"; }
var v15 = 15;
if (v15 !== 15) { throw "v15"; }
var v16 = 16;
if (v16 !== 16) { throw "v16"; }
var v17 = 17;
if (v17 !== 17) { throw "v17"; }
if ((v17 + 1) !== 18) { throw "branch17"; }
var v18 = 18;
if (v18 !== 18) { throw "v18"; }
var v19 = 19;
if (v19 !== 19) { throw "v19"; }
var v20 = 20;
if (v20 !== 20) { throw "v20"; }
var v21 = 21;
if (v21 !== 21) { throw "v21"; }
var v22 = 22;
if (v22 !== 22) { throw "v22"; }
var v23 = 23;
if (v23 !== 23) { throw "v23"; }
var v24 = 24;
if (v24 !== 24) { throw "v24"; }
var v25 = 25;
if (v25 !== 25) { throw "v25"; }
var v26 = 26;
if (v26 !== 26) { throw "v26"; }
var v27 = 27;
if (v27 !== 27) { throw "v27"; }
var v28 = 28;
if (v28 !== 28) { throw "v28"; }
var v29 = 29;
if (v29 !== 29) { throw "v29"; }
var v30 = 30;
if (v30 !== 30) { throw "v30"; }
var v31 = 31;
if (v31 !== 31) { throw "v31"; }
var v32 = 32;
if (v32 !== 32) { throw "v32"; }
var v33 = 33;
if (v33 !== 33) { throw "v33"; }
var v34 = 34;
if (v34 !== 34) { throw "v34"; }
if ((v34 + 1) !== 35) { throw "branch34"; }
var v35 = 35;
if (v35 !== 35) { throw "v35"; }
var v36 = 36;
if (v36 !== 36) { throw "v36"; }
var v37 = 37;
if (v37 !== 37) { throw "v37"; }
var v38 = 38;
if (v38 !== 38) { throw "v38"; }
var v39 = 39;
if (v39 !== 39) { throw "v39"; }
var v40 = 40;
if (v40 !== 40) { throw "v40"; }
var v41 = 41;
if (v41 !== 41) { throw "v41"; }
var v42 = 42;
if (v42 !== 42) { throw "v42"; }
var v43 = 43;
if (v43 !== 43) { throw "v43"; }
return privateBox.getValue();
}
if (__ayyRun() !== 37) { throw "result"; }
