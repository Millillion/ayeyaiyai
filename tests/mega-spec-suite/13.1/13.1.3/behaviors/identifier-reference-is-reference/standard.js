// behavior: identifier-reference-is-reference
// expected: pass
// goal: script
// size: standard
// variant: script.sloppy

var ordinaryReference = 1;
var yield = 10;
var await = 20;
function __ayyRun() {
var score = 0;
ordinaryReference = ordinaryReference + 2;
if (ordinaryReference !== 3) { throw "ordinary assignment"; }
yield = yield + ordinaryReference;
if (yield !== 13) { throw "yield assignment"; }
await = await + yield;
if (await !== 33) { throw "await assignment"; }
var localReference = 5;
localReference = localReference + ordinaryReference + yield;
if (localReference !== 21) { throw "local assignment"; }
score = score + ordinaryReference + yield + await + localReference;
var ref0 = 0;
ref0 = ref0 + ordinaryReference;
if (ref0 !== 3) { throw "ref 0"; }
score = score + ref0;
yield = yield + 1;
yield = yield - 1;
if (yield !== 13) { throw "yield stable 0"; }
await = await + 2;
await = await - 2;
if (await !== 33) { throw "await stable 0"; }
var ref1 = 1;
ref1 = ref1 + ordinaryReference;
if (ref1 !== 4) { throw "ref 1"; }
score = score + ref1;
var ref2 = 2;
ref2 = ref2 + ordinaryReference;
if (ref2 !== 5) { throw "ref 2"; }
score = score + ref2;
var ref3 = 3;
ref3 = ref3 + ordinaryReference;
if (ref3 !== 6) { throw "ref 3"; }
score = score + ref3;
var ref4 = 4;
ref4 = ref4 + ordinaryReference;
if (ref4 !== 7) { throw "ref 4"; }
score = score + ref4;
var ref5 = 5;
ref5 = ref5 + ordinaryReference;
if (ref5 !== 8) { throw "ref 5"; }
score = score + ref5;
var ref6 = 6;
ref6 = ref6 + ordinaryReference;
if (ref6 !== 9) { throw "ref 6"; }
score = score + ref6;
var ref7 = 7;
ref7 = ref7 + ordinaryReference;
if (ref7 !== 10) { throw "ref 7"; }
score = score + ref7;
var ref8 = 8;
ref8 = ref8 + ordinaryReference;
if (ref8 !== 11) { throw "ref 8"; }
score = score + ref8;
var ref9 = 9;
ref9 = ref9 + ordinaryReference;
if (ref9 !== 12) { throw "ref 9"; }
score = score + ref9;
var ref10 = 10;
ref10 = ref10 + ordinaryReference;
if (ref10 !== 13) { throw "ref 10"; }
score = score + ref10;
var ref11 = 11;
ref11 = ref11 + ordinaryReference;
if (ref11 !== 14) { throw "ref 11"; }
score = score + ref11;
var ref12 = 12;
ref12 = ref12 + ordinaryReference;
if (ref12 !== 15) { throw "ref 12"; }
score = score + ref12;
var ref13 = 13;
ref13 = ref13 + ordinaryReference;
if (ref13 !== 16) { throw "ref 13"; }
score = score + ref13;
var ref14 = 14;
ref14 = ref14 + ordinaryReference;
if (ref14 !== 17) { throw "ref 14"; }
score = score + ref14;
var ref15 = 15;
ref15 = ref15 + ordinaryReference;
if (ref15 !== 18) { throw "ref 15"; }
score = score + ref15;
var ref16 = 16;
ref16 = ref16 + ordinaryReference;
if (ref16 !== 19) { throw "ref 16"; }
score = score + ref16;
var ref17 = 17;
ref17 = ref17 + ordinaryReference;
if (ref17 !== 20) { throw "ref 17"; }
score = score + ref17;
yield = yield + 1;
yield = yield - 1;
if (yield !== 13) { throw "yield stable 17"; }
score = score + 0;
return score;
}
console.log("ok", __ayyRun(), ordinaryReference, yield, await);
