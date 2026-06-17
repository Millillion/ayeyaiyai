// behavior: identifier-reference-is-reference
// expected: pass
// goal: script
// size: large
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
var ref18 = 18;
ref18 = ref18 + ordinaryReference;
if (ref18 !== 21) { throw "ref 18"; }
score = score + ref18;
var ref19 = 19;
ref19 = ref19 + ordinaryReference;
if (ref19 !== 22) { throw "ref 19"; }
score = score + ref19;
var ref20 = 20;
ref20 = ref20 + ordinaryReference;
if (ref20 !== 23) { throw "ref 20"; }
score = score + ref20;
var ref21 = 21;
ref21 = ref21 + ordinaryReference;
if (ref21 !== 24) { throw "ref 21"; }
score = score + ref21;
var ref22 = 22;
ref22 = ref22 + ordinaryReference;
if (ref22 !== 25) { throw "ref 22"; }
score = score + ref22;
var ref23 = 23;
ref23 = ref23 + ordinaryReference;
if (ref23 !== 26) { throw "ref 23"; }
score = score + ref23;
await = await + 2;
await = await - 2;
if (await !== 33) { throw "await stable 23"; }
var ref24 = 24;
ref24 = ref24 + ordinaryReference;
if (ref24 !== 27) { throw "ref 24"; }
score = score + ref24;
var ref25 = 25;
ref25 = ref25 + ordinaryReference;
if (ref25 !== 28) { throw "ref 25"; }
score = score + ref25;
var ref26 = 26;
ref26 = ref26 + ordinaryReference;
if (ref26 !== 29) { throw "ref 26"; }
score = score + ref26;
var ref27 = 27;
ref27 = ref27 + ordinaryReference;
if (ref27 !== 30) { throw "ref 27"; }
score = score + ref27;
var ref28 = 28;
ref28 = ref28 + ordinaryReference;
if (ref28 !== 31) { throw "ref 28"; }
score = score + ref28;
var ref29 = 29;
ref29 = ref29 + ordinaryReference;
if (ref29 !== 32) { throw "ref 29"; }
score = score + ref29;
var ref30 = 30;
ref30 = ref30 + ordinaryReference;
if (ref30 !== 33) { throw "ref 30"; }
score = score + ref30;
var ref31 = 31;
ref31 = ref31 + ordinaryReference;
if (ref31 !== 34) { throw "ref 31"; }
score = score + ref31;
var ref32 = 32;
ref32 = ref32 + ordinaryReference;
if (ref32 !== 35) { throw "ref 32"; }
score = score + ref32;
var ref33 = 33;
ref33 = ref33 + ordinaryReference;
if (ref33 !== 36) { throw "ref 33"; }
score = score + ref33;
var ref34 = 34;
ref34 = ref34 + ordinaryReference;
if (ref34 !== 37) { throw "ref 34"; }
score = score + ref34;
yield = yield + 1;
yield = yield - 1;
if (yield !== 13) { throw "yield stable 34"; }
var ref35 = 35;
ref35 = ref35 + ordinaryReference;
if (ref35 !== 38) { throw "ref 35"; }
score = score + ref35;
var ref36 = 36;
ref36 = ref36 + ordinaryReference;
if (ref36 !== 39) { throw "ref 36"; }
score = score + ref36;
var ref37 = 37;
ref37 = ref37 + ordinaryReference;
if (ref37 !== 40) { throw "ref 37"; }
score = score + ref37;
var ref38 = 38;
ref38 = ref38 + ordinaryReference;
if (ref38 !== 41) { throw "ref 38"; }
score = score + ref38;
var ref39 = 39;
ref39 = ref39 + ordinaryReference;
if (ref39 !== 42) { throw "ref 39"; }
score = score + ref39;
var ref40 = 40;
ref40 = ref40 + ordinaryReference;
if (ref40 !== 43) { throw "ref 40"; }
score = score + ref40;
var ref41 = 41;
ref41 = ref41 + ordinaryReference;
if (ref41 !== 44) { throw "ref 41"; }
score = score + ref41;
var ref42 = 42;
ref42 = ref42 + ordinaryReference;
if (ref42 !== 45) { throw "ref 42"; }
score = score + ref42;
var ref43 = 43;
ref43 = ref43 + ordinaryReference;
if (ref43 !== 46) { throw "ref 43"; }
score = score + ref43;
var ref44 = 44;
ref44 = ref44 + ordinaryReference;
if (ref44 !== 47) { throw "ref 44"; }
score = score + ref44;
var ref45 = 45;
ref45 = ref45 + ordinaryReference;
if (ref45 !== 48) { throw "ref 45"; }
score = score + ref45;
var ref46 = 46;
ref46 = ref46 + ordinaryReference;
if (ref46 !== 49) { throw "ref 46"; }
score = score + ref46;
await = await + 2;
await = await - 2;
if (await !== 33) { throw "await stable 46"; }
var ref47 = 47;
ref47 = ref47 + ordinaryReference;
if (ref47 !== 50) { throw "ref 47"; }
score = score + ref47;
var ref48 = 48;
ref48 = ref48 + ordinaryReference;
if (ref48 !== 51) { throw "ref 48"; }
score = score + ref48;
var ref49 = 49;
ref49 = ref49 + ordinaryReference;
if (ref49 !== 52) { throw "ref 49"; }
score = score + ref49;
var ref50 = 50;
ref50 = ref50 + ordinaryReference;
if (ref50 !== 53) { throw "ref 50"; }
score = score + ref50;
var ref51 = 51;
ref51 = ref51 + ordinaryReference;
if (ref51 !== 54) { throw "ref 51"; }
score = score + ref51;
yield = yield + 1;
yield = yield - 1;
if (yield !== 13) { throw "yield stable 51"; }
var ref52 = 52;
ref52 = ref52 + ordinaryReference;
if (ref52 !== 55) { throw "ref 52"; }
score = score + ref52;
var ref53 = 53;
ref53 = ref53 + ordinaryReference;
if (ref53 !== 56) { throw "ref 53"; }
score = score + ref53;
var ref54 = 54;
ref54 = ref54 + ordinaryReference;
if (ref54 !== 57) { throw "ref 54"; }
score = score + ref54;
var ref55 = 55;
ref55 = ref55 + ordinaryReference;
if (ref55 !== 58) { throw "ref 55"; }
score = score + ref55;
var ref56 = 56;
ref56 = ref56 + ordinaryReference;
if (ref56 !== 59) { throw "ref 56"; }
score = score + ref56;
var ref57 = 57;
ref57 = ref57 + ordinaryReference;
if (ref57 !== 60) { throw "ref 57"; }
score = score + ref57;
var ref58 = 58;
ref58 = ref58 + ordinaryReference;
if (ref58 !== 61) { throw "ref 58"; }
score = score + ref58;
var ref59 = 59;
ref59 = ref59 + ordinaryReference;
if (ref59 !== 62) { throw "ref 59"; }
score = score + ref59;
var ref60 = 60;
ref60 = ref60 + ordinaryReference;
if (ref60 !== 63) { throw "ref 60"; }
score = score + ref60;
var ref61 = 61;
ref61 = ref61 + ordinaryReference;
if (ref61 !== 64) { throw "ref 61"; }
score = score + ref61;
var ref62 = 62;
ref62 = ref62 + ordinaryReference;
if (ref62 !== 65) { throw "ref 62"; }
score = score + ref62;
var ref63 = 63;
ref63 = ref63 + ordinaryReference;
if (ref63 !== 66) { throw "ref 63"; }
score = score + ref63;
var ref64 = 64;
ref64 = ref64 + ordinaryReference;
if (ref64 !== 67) { throw "ref 64"; }
score = score + ref64;
var ref65 = 65;
ref65 = ref65 + ordinaryReference;
if (ref65 !== 68) { throw "ref 65"; }
score = score + ref65;
var ref66 = 66;
ref66 = ref66 + ordinaryReference;
if (ref66 !== 69) { throw "ref 66"; }
score = score + ref66;
var ref67 = 67;
ref67 = ref67 + ordinaryReference;
if (ref67 !== 70) { throw "ref 67"; }
score = score + ref67;
var ref68 = 68;
ref68 = ref68 + ordinaryReference;
if (ref68 !== 71) { throw "ref 68"; }
score = score + ref68;
yield = yield + 1;
yield = yield - 1;
if (yield !== 13) { throw "yield stable 68"; }
var ref69 = 69;
ref69 = ref69 + ordinaryReference;
if (ref69 !== 72) { throw "ref 69"; }
score = score + ref69;
await = await + 2;
await = await - 2;
if (await !== 33) { throw "await stable 69"; }
var ref70 = 70;
ref70 = ref70 + ordinaryReference;
if (ref70 !== 73) { throw "ref 70"; }
score = score + ref70;
var ref71 = 71;
ref71 = ref71 + ordinaryReference;
if (ref71 !== 74) { throw "ref 71"; }
score = score + ref71;
var ref72 = 72;
ref72 = ref72 + ordinaryReference;
if (ref72 !== 75) { throw "ref 72"; }
score = score + ref72;
var ref73 = 73;
ref73 = ref73 + ordinaryReference;
if (ref73 !== 76) { throw "ref 73"; }
score = score + ref73;
var ref74 = 74;
ref74 = ref74 + ordinaryReference;
if (ref74 !== 77) { throw "ref 74"; }
score = score + ref74;
var ref75 = 75;
ref75 = ref75 + ordinaryReference;
if (ref75 !== 78) { throw "ref 75"; }
score = score + ref75;
var ref76 = 76;
ref76 = ref76 + ordinaryReference;
if (ref76 !== 79) { throw "ref 76"; }
score = score + ref76;
var ref77 = 77;
ref77 = ref77 + ordinaryReference;
if (ref77 !== 80) { throw "ref 77"; }
score = score + ref77;
var ref78 = 78;
ref78 = ref78 + ordinaryReference;
if (ref78 !== 81) { throw "ref 78"; }
score = score + ref78;
var ref79 = 79;
ref79 = ref79 + ordinaryReference;
if (ref79 !== 82) { throw "ref 79"; }
score = score + ref79;
var ref80 = 80;
ref80 = ref80 + ordinaryReference;
if (ref80 !== 83) { throw "ref 80"; }
score = score + ref80;
var ref81 = 81;
ref81 = ref81 + ordinaryReference;
if (ref81 !== 84) { throw "ref 81"; }
score = score + ref81;
var ref82 = 82;
ref82 = ref82 + ordinaryReference;
if (ref82 !== 85) { throw "ref 82"; }
score = score + ref82;
var ref83 = 83;
ref83 = ref83 + ordinaryReference;
if (ref83 !== 86) { throw "ref 83"; }
score = score + ref83;
var ref84 = 84;
ref84 = ref84 + ordinaryReference;
if (ref84 !== 87) { throw "ref 84"; }
score = score + ref84;
var ref85 = 85;
ref85 = ref85 + ordinaryReference;
if (ref85 !== 88) { throw "ref 85"; }
score = score + ref85;
yield = yield + 1;
yield = yield - 1;
if (yield !== 13) { throw "yield stable 85"; }
var ref86 = 86;
ref86 = ref86 + ordinaryReference;
if (ref86 !== 89) { throw "ref 86"; }
score = score + ref86;
var ref87 = 87;
ref87 = ref87 + ordinaryReference;
if (ref87 !== 90) { throw "ref 87"; }
score = score + ref87;
var ref88 = 88;
ref88 = ref88 + ordinaryReference;
if (ref88 !== 91) { throw "ref 88"; }
score = score + ref88;
var ref89 = 89;
ref89 = ref89 + ordinaryReference;
if (ref89 !== 92) { throw "ref 89"; }
score = score + ref89;
var ref90 = 90;
ref90 = ref90 + ordinaryReference;
if (ref90 !== 93) { throw "ref 90"; }
score = score + ref90;
var ref91 = 91;
ref91 = ref91 + ordinaryReference;
if (ref91 !== 94) { throw "ref 91"; }
score = score + ref91;
var ref92 = 92;
ref92 = ref92 + ordinaryReference;
if (ref92 !== 95) { throw "ref 92"; }
score = score + ref92;
await = await + 2;
await = await - 2;
if (await !== 33) { throw "await stable 92"; }
var ref93 = 93;
ref93 = ref93 + ordinaryReference;
if (ref93 !== 96) { throw "ref 93"; }
score = score + ref93;
var ref94 = 94;
ref94 = ref94 + ordinaryReference;
if (ref94 !== 97) { throw "ref 94"; }
score = score + ref94;
var ref95 = 95;
ref95 = ref95 + ordinaryReference;
if (ref95 !== 98) { throw "ref 95"; }
score = score + ref95;
var ref96 = 96;
ref96 = ref96 + ordinaryReference;
if (ref96 !== 99) { throw "ref 96"; }
score = score + ref96;
var ref97 = 97;
ref97 = ref97 + ordinaryReference;
if (ref97 !== 100) { throw "ref 97"; }
score = score + ref97;
var ref98 = 98;
ref98 = ref98 + ordinaryReference;
if (ref98 !== 101) { throw "ref 98"; }
score = score + ref98;
var ref99 = 99;
ref99 = ref99 + ordinaryReference;
if (ref99 !== 102) { throw "ref 99"; }
score = score + ref99;
var ref100 = 100;
ref100 = ref100 + ordinaryReference;
if (ref100 !== 103) { throw "ref 100"; }
score = score + ref100;
var ref101 = 101;
ref101 = ref101 + ordinaryReference;
if (ref101 !== 104) { throw "ref 101"; }
score = score + ref101;
var ref102 = 102;
ref102 = ref102 + ordinaryReference;
if (ref102 !== 105) { throw "ref 102"; }
score = score + ref102;
yield = yield + 1;
yield = yield - 1;
if (yield !== 13) { throw "yield stable 102"; }
var ref103 = 103;
ref103 = ref103 + ordinaryReference;
if (ref103 !== 106) { throw "ref 103"; }
score = score + ref103;
var ref104 = 104;
ref104 = ref104 + ordinaryReference;
if (ref104 !== 107) { throw "ref 104"; }
score = score + ref104;
var ref105 = 105;
ref105 = ref105 + ordinaryReference;
if (ref105 !== 108) { throw "ref 105"; }
score = score + ref105;
var ref106 = 106;
ref106 = ref106 + ordinaryReference;
if (ref106 !== 109) { throw "ref 106"; }
score = score + ref106;
var ref107 = 107;
ref107 = ref107 + ordinaryReference;
if (ref107 !== 110) { throw "ref 107"; }
score = score + ref107;
var ref108 = 108;
ref108 = ref108 + ordinaryReference;
if (ref108 !== 111) { throw "ref 108"; }
score = score + ref108;
var ref109 = 109;
ref109 = ref109 + ordinaryReference;
if (ref109 !== 112) { throw "ref 109"; }
score = score + ref109;
var ref110 = 110;
ref110 = ref110 + ordinaryReference;
if (ref110 !== 113) { throw "ref 110"; }
score = score + ref110;
var ref111 = 111;
ref111 = ref111 + ordinaryReference;
if (ref111 !== 114) { throw "ref 111"; }
score = score + ref111;
return score;
}
console.log("ok", __ayyRun(), ordinaryReference, yield, await);
