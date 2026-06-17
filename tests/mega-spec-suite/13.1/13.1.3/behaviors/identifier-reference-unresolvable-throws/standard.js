// behavior: identifier-reference-unresolvable-throws
// expected: runtime-error
// goal: script
// size: standard
// variant: script.sloppy

var setupTotal = 0;
function __ayySetup() {
var localTotal = 0;
var setup0 = 0;
localTotal = localTotal + setup0;
if (setup0 !== 0) { throw "setup 0"; }
var setupCheck0 = setup0 + 1;
if (setupCheck0 !== 1) { throw "setup check 0"; }
var setup1 = 1;
localTotal = localTotal + setup1;
if (setup1 !== 1) { throw "setup 1"; }
var setup2 = 2;
localTotal = localTotal + setup2;
if (setup2 !== 2) { throw "setup 2"; }
var setup3 = 3;
localTotal = localTotal + setup3;
if (setup3 !== 3) { throw "setup 3"; }
var setup4 = 4;
localTotal = localTotal + setup4;
if (setup4 !== 4) { throw "setup 4"; }
var setup5 = 5;
localTotal = localTotal + setup5;
if (setup5 !== 5) { throw "setup 5"; }
var setup6 = 6;
localTotal = localTotal + setup6;
if (setup6 !== 6) { throw "setup 6"; }
var setup7 = 7;
localTotal = localTotal + setup7;
if (setup7 !== 7) { throw "setup 7"; }
var setup8 = 8;
localTotal = localTotal + setup8;
if (setup8 !== 8) { throw "setup 8"; }
var setup9 = 9;
localTotal = localTotal + setup9;
if (setup9 !== 9) { throw "setup 9"; }
var setup10 = 10;
localTotal = localTotal + setup10;
if (setup10 !== 10) { throw "setup 10"; }
var setup11 = 11;
localTotal = localTotal + setup11;
if (setup11 !== 11) { throw "setup 11"; }
var setup12 = 12;
localTotal = localTotal + setup12;
if (setup12 !== 12) { throw "setup 12"; }
var setup13 = 13;
localTotal = localTotal + setup13;
if (setup13 !== 13) { throw "setup 13"; }
var setup14 = 14;
localTotal = localTotal + setup14;
if (setup14 !== 14) { throw "setup 14"; }
var setup15 = 15;
localTotal = localTotal + setup15;
if (setup15 !== 15) { throw "setup 15"; }
var setup16 = 16;
localTotal = localTotal + setup16;
if (setup16 !== 16) { throw "setup 16"; }
var setup17 = 17;
localTotal = localTotal + setup17;
if (setup17 !== 17) { throw "setup 17"; }
var setup18 = 18;
localTotal = localTotal + setup18;
if (setup18 !== 18) { throw "setup 18"; }
var setup19 = 19;
localTotal = localTotal + setup19;
if (setup19 !== 19) { throw "setup 19"; }
var setup20 = 20;
localTotal = localTotal + setup20;
if (setup20 !== 20) { throw "setup 20"; }
var setup21 = 21;
localTotal = localTotal + setup21;
if (setup21 !== 21) { throw "setup 21"; }
var setup22 = 22;
localTotal = localTotal + setup22;
if (setup22 !== 22) { throw "setup 22"; }
var setup23 = 23;
localTotal = localTotal + setup23;
if (setup23 !== 23) { throw "setup 23"; }
var setup24 = 24;
localTotal = localTotal + setup24;
if (setup24 !== 24) { throw "setup 24"; }
var setup25 = 25;
localTotal = localTotal + setup25;
if (setup25 !== 25) { throw "setup 25"; }
var setup26 = 26;
localTotal = localTotal + setup26;
if (setup26 !== 26) { throw "setup 26"; }
var setup27 = 27;
localTotal = localTotal + setup27;
if (setup27 !== 27) { throw "setup 27"; }
var setup28 = 28;
localTotal = localTotal + setup28;
if (setup28 !== 28) { throw "setup 28"; }
var setup29 = 29;
localTotal = localTotal + setup29;
if (setup29 !== 29) { throw "setup 29"; }
return localTotal;
}
setupTotal = __ayySetup();
if (setupTotal < 0) { throw "setup total"; }
console.log(__ayyDefinitelyMissingIdentifier);
