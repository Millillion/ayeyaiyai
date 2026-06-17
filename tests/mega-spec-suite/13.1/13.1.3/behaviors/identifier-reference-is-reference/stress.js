// behavior: identifier-reference-is-reference
// expected: pass
// goal: script
// size: stress
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
var ref112 = 112;
ref112 = ref112 + ordinaryReference;
if (ref112 !== 115) { throw "ref 112"; }
score = score + ref112;
var ref113 = 113;
ref113 = ref113 + ordinaryReference;
if (ref113 !== 116) { throw "ref 113"; }
score = score + ref113;
var ref114 = 114;
ref114 = ref114 + ordinaryReference;
if (ref114 !== 117) { throw "ref 114"; }
score = score + ref114;
var ref115 = 115;
ref115 = ref115 + ordinaryReference;
if (ref115 !== 118) { throw "ref 115"; }
score = score + ref115;
await = await + 2;
await = await - 2;
if (await !== 33) { throw "await stable 115"; }
var ref116 = 116;
ref116 = ref116 + ordinaryReference;
if (ref116 !== 119) { throw "ref 116"; }
score = score + ref116;
var ref117 = 117;
ref117 = ref117 + ordinaryReference;
if (ref117 !== 120) { throw "ref 117"; }
score = score + ref117;
var ref118 = 118;
ref118 = ref118 + ordinaryReference;
if (ref118 !== 121) { throw "ref 118"; }
score = score + ref118;
var ref119 = 119;
ref119 = ref119 + ordinaryReference;
if (ref119 !== 122) { throw "ref 119"; }
score = score + ref119;
yield = yield + 1;
yield = yield - 1;
if (yield !== 13) { throw "yield stable 119"; }
var ref120 = 120;
ref120 = ref120 + ordinaryReference;
if (ref120 !== 123) { throw "ref 120"; }
score = score + ref120;
var ref121 = 121;
ref121 = ref121 + ordinaryReference;
if (ref121 !== 124) { throw "ref 121"; }
score = score + ref121;
var ref122 = 122;
ref122 = ref122 + ordinaryReference;
if (ref122 !== 125) { throw "ref 122"; }
score = score + ref122;
var ref123 = 123;
ref123 = ref123 + ordinaryReference;
if (ref123 !== 126) { throw "ref 123"; }
score = score + ref123;
var ref124 = 124;
ref124 = ref124 + ordinaryReference;
if (ref124 !== 127) { throw "ref 124"; }
score = score + ref124;
var ref125 = 125;
ref125 = ref125 + ordinaryReference;
if (ref125 !== 128) { throw "ref 125"; }
score = score + ref125;
var ref126 = 126;
ref126 = ref126 + ordinaryReference;
if (ref126 !== 129) { throw "ref 126"; }
score = score + ref126;
var ref127 = 127;
ref127 = ref127 + ordinaryReference;
if (ref127 !== 130) { throw "ref 127"; }
score = score + ref127;
var ref128 = 128;
ref128 = ref128 + ordinaryReference;
if (ref128 !== 131) { throw "ref 128"; }
score = score + ref128;
var ref129 = 129;
ref129 = ref129 + ordinaryReference;
if (ref129 !== 132) { throw "ref 129"; }
score = score + ref129;
var ref130 = 130;
ref130 = ref130 + ordinaryReference;
if (ref130 !== 133) { throw "ref 130"; }
score = score + ref130;
var ref131 = 131;
ref131 = ref131 + ordinaryReference;
if (ref131 !== 134) { throw "ref 131"; }
score = score + ref131;
var ref132 = 132;
ref132 = ref132 + ordinaryReference;
if (ref132 !== 135) { throw "ref 132"; }
score = score + ref132;
var ref133 = 133;
ref133 = ref133 + ordinaryReference;
if (ref133 !== 136) { throw "ref 133"; }
score = score + ref133;
var ref134 = 134;
ref134 = ref134 + ordinaryReference;
if (ref134 !== 137) { throw "ref 134"; }
score = score + ref134;
var ref135 = 135;
ref135 = ref135 + ordinaryReference;
if (ref135 !== 138) { throw "ref 135"; }
score = score + ref135;
var ref136 = 136;
ref136 = ref136 + ordinaryReference;
if (ref136 !== 139) { throw "ref 136"; }
score = score + ref136;
yield = yield + 1;
yield = yield - 1;
if (yield !== 13) { throw "yield stable 136"; }
var ref137 = 137;
ref137 = ref137 + ordinaryReference;
if (ref137 !== 140) { throw "ref 137"; }
score = score + ref137;
var ref138 = 138;
ref138 = ref138 + ordinaryReference;
if (ref138 !== 141) { throw "ref 138"; }
score = score + ref138;
await = await + 2;
await = await - 2;
if (await !== 33) { throw "await stable 138"; }
var ref139 = 139;
ref139 = ref139 + ordinaryReference;
if (ref139 !== 142) { throw "ref 139"; }
score = score + ref139;
var ref140 = 140;
ref140 = ref140 + ordinaryReference;
if (ref140 !== 143) { throw "ref 140"; }
score = score + ref140;
var ref141 = 141;
ref141 = ref141 + ordinaryReference;
if (ref141 !== 144) { throw "ref 141"; }
score = score + ref141;
var ref142 = 142;
ref142 = ref142 + ordinaryReference;
if (ref142 !== 145) { throw "ref 142"; }
score = score + ref142;
var ref143 = 143;
ref143 = ref143 + ordinaryReference;
if (ref143 !== 146) { throw "ref 143"; }
score = score + ref143;
var ref144 = 144;
ref144 = ref144 + ordinaryReference;
if (ref144 !== 147) { throw "ref 144"; }
score = score + ref144;
var ref145 = 145;
ref145 = ref145 + ordinaryReference;
if (ref145 !== 148) { throw "ref 145"; }
score = score + ref145;
var ref146 = 146;
ref146 = ref146 + ordinaryReference;
if (ref146 !== 149) { throw "ref 146"; }
score = score + ref146;
var ref147 = 147;
ref147 = ref147 + ordinaryReference;
if (ref147 !== 150) { throw "ref 147"; }
score = score + ref147;
var ref148 = 148;
ref148 = ref148 + ordinaryReference;
if (ref148 !== 151) { throw "ref 148"; }
score = score + ref148;
var ref149 = 149;
ref149 = ref149 + ordinaryReference;
if (ref149 !== 152) { throw "ref 149"; }
score = score + ref149;
var ref150 = 150;
ref150 = ref150 + ordinaryReference;
if (ref150 !== 153) { throw "ref 150"; }
score = score + ref150;
var ref151 = 151;
ref151 = ref151 + ordinaryReference;
if (ref151 !== 154) { throw "ref 151"; }
score = score + ref151;
var ref152 = 152;
ref152 = ref152 + ordinaryReference;
if (ref152 !== 155) { throw "ref 152"; }
score = score + ref152;
var ref153 = 153;
ref153 = ref153 + ordinaryReference;
if (ref153 !== 156) { throw "ref 153"; }
score = score + ref153;
yield = yield + 1;
yield = yield - 1;
if (yield !== 13) { throw "yield stable 153"; }
var ref154 = 154;
ref154 = ref154 + ordinaryReference;
if (ref154 !== 157) { throw "ref 154"; }
score = score + ref154;
var ref155 = 155;
ref155 = ref155 + ordinaryReference;
if (ref155 !== 158) { throw "ref 155"; }
score = score + ref155;
var ref156 = 156;
ref156 = ref156 + ordinaryReference;
if (ref156 !== 159) { throw "ref 156"; }
score = score + ref156;
var ref157 = 157;
ref157 = ref157 + ordinaryReference;
if (ref157 !== 160) { throw "ref 157"; }
score = score + ref157;
var ref158 = 158;
ref158 = ref158 + ordinaryReference;
if (ref158 !== 161) { throw "ref 158"; }
score = score + ref158;
var ref159 = 159;
ref159 = ref159 + ordinaryReference;
if (ref159 !== 162) { throw "ref 159"; }
score = score + ref159;
var ref160 = 160;
ref160 = ref160 + ordinaryReference;
if (ref160 !== 163) { throw "ref 160"; }
score = score + ref160;
var ref161 = 161;
ref161 = ref161 + ordinaryReference;
if (ref161 !== 164) { throw "ref 161"; }
score = score + ref161;
await = await + 2;
await = await - 2;
if (await !== 33) { throw "await stable 161"; }
var ref162 = 162;
ref162 = ref162 + ordinaryReference;
if (ref162 !== 165) { throw "ref 162"; }
score = score + ref162;
var ref163 = 163;
ref163 = ref163 + ordinaryReference;
if (ref163 !== 166) { throw "ref 163"; }
score = score + ref163;
var ref164 = 164;
ref164 = ref164 + ordinaryReference;
if (ref164 !== 167) { throw "ref 164"; }
score = score + ref164;
var ref165 = 165;
ref165 = ref165 + ordinaryReference;
if (ref165 !== 168) { throw "ref 165"; }
score = score + ref165;
var ref166 = 166;
ref166 = ref166 + ordinaryReference;
if (ref166 !== 169) { throw "ref 166"; }
score = score + ref166;
var ref167 = 167;
ref167 = ref167 + ordinaryReference;
if (ref167 !== 170) { throw "ref 167"; }
score = score + ref167;
var ref168 = 168;
ref168 = ref168 + ordinaryReference;
if (ref168 !== 171) { throw "ref 168"; }
score = score + ref168;
var ref169 = 169;
ref169 = ref169 + ordinaryReference;
if (ref169 !== 172) { throw "ref 169"; }
score = score + ref169;
var ref170 = 170;
ref170 = ref170 + ordinaryReference;
if (ref170 !== 173) { throw "ref 170"; }
score = score + ref170;
yield = yield + 1;
yield = yield - 1;
if (yield !== 13) { throw "yield stable 170"; }
var ref171 = 171;
ref171 = ref171 + ordinaryReference;
if (ref171 !== 174) { throw "ref 171"; }
score = score + ref171;
var ref172 = 172;
ref172 = ref172 + ordinaryReference;
if (ref172 !== 175) { throw "ref 172"; }
score = score + ref172;
var ref173 = 173;
ref173 = ref173 + ordinaryReference;
if (ref173 !== 176) { throw "ref 173"; }
score = score + ref173;
var ref174 = 174;
ref174 = ref174 + ordinaryReference;
if (ref174 !== 177) { throw "ref 174"; }
score = score + ref174;
var ref175 = 175;
ref175 = ref175 + ordinaryReference;
if (ref175 !== 178) { throw "ref 175"; }
score = score + ref175;
var ref176 = 176;
ref176 = ref176 + ordinaryReference;
if (ref176 !== 179) { throw "ref 176"; }
score = score + ref176;
var ref177 = 177;
ref177 = ref177 + ordinaryReference;
if (ref177 !== 180) { throw "ref 177"; }
score = score + ref177;
var ref178 = 178;
ref178 = ref178 + ordinaryReference;
if (ref178 !== 181) { throw "ref 178"; }
score = score + ref178;
var ref179 = 179;
ref179 = ref179 + ordinaryReference;
if (ref179 !== 182) { throw "ref 179"; }
score = score + ref179;
var ref180 = 180;
ref180 = ref180 + ordinaryReference;
if (ref180 !== 183) { throw "ref 180"; }
score = score + ref180;
var ref181 = 181;
ref181 = ref181 + ordinaryReference;
if (ref181 !== 184) { throw "ref 181"; }
score = score + ref181;
var ref182 = 182;
ref182 = ref182 + ordinaryReference;
if (ref182 !== 185) { throw "ref 182"; }
score = score + ref182;
var ref183 = 183;
ref183 = ref183 + ordinaryReference;
if (ref183 !== 186) { throw "ref 183"; }
score = score + ref183;
var ref184 = 184;
ref184 = ref184 + ordinaryReference;
if (ref184 !== 187) { throw "ref 184"; }
score = score + ref184;
await = await + 2;
await = await - 2;
if (await !== 33) { throw "await stable 184"; }
var ref185 = 185;
ref185 = ref185 + ordinaryReference;
if (ref185 !== 188) { throw "ref 185"; }
score = score + ref185;
var ref186 = 186;
ref186 = ref186 + ordinaryReference;
if (ref186 !== 189) { throw "ref 186"; }
score = score + ref186;
var ref187 = 187;
ref187 = ref187 + ordinaryReference;
if (ref187 !== 190) { throw "ref 187"; }
score = score + ref187;
yield = yield + 1;
yield = yield - 1;
if (yield !== 13) { throw "yield stable 187"; }
var ref188 = 188;
ref188 = ref188 + ordinaryReference;
if (ref188 !== 191) { throw "ref 188"; }
score = score + ref188;
var ref189 = 189;
ref189 = ref189 + ordinaryReference;
if (ref189 !== 192) { throw "ref 189"; }
score = score + ref189;
var ref190 = 190;
ref190 = ref190 + ordinaryReference;
if (ref190 !== 193) { throw "ref 190"; }
score = score + ref190;
var ref191 = 191;
ref191 = ref191 + ordinaryReference;
if (ref191 !== 194) { throw "ref 191"; }
score = score + ref191;
var ref192 = 192;
ref192 = ref192 + ordinaryReference;
if (ref192 !== 195) { throw "ref 192"; }
score = score + ref192;
var ref193 = 193;
ref193 = ref193 + ordinaryReference;
if (ref193 !== 196) { throw "ref 193"; }
score = score + ref193;
var ref194 = 194;
ref194 = ref194 + ordinaryReference;
if (ref194 !== 197) { throw "ref 194"; }
score = score + ref194;
var ref195 = 195;
ref195 = ref195 + ordinaryReference;
if (ref195 !== 198) { throw "ref 195"; }
score = score + ref195;
var ref196 = 196;
ref196 = ref196 + ordinaryReference;
if (ref196 !== 199) { throw "ref 196"; }
score = score + ref196;
var ref197 = 197;
ref197 = ref197 + ordinaryReference;
if (ref197 !== 200) { throw "ref 197"; }
score = score + ref197;
var ref198 = 198;
ref198 = ref198 + ordinaryReference;
if (ref198 !== 201) { throw "ref 198"; }
score = score + ref198;
var ref199 = 199;
ref199 = ref199 + ordinaryReference;
if (ref199 !== 202) { throw "ref 199"; }
score = score + ref199;
var ref200 = 200;
ref200 = ref200 + ordinaryReference;
if (ref200 !== 203) { throw "ref 200"; }
score = score + ref200;
var ref201 = 201;
ref201 = ref201 + ordinaryReference;
if (ref201 !== 204) { throw "ref 201"; }
score = score + ref201;
var ref202 = 202;
ref202 = ref202 + ordinaryReference;
if (ref202 !== 205) { throw "ref 202"; }
score = score + ref202;
var ref203 = 203;
ref203 = ref203 + ordinaryReference;
if (ref203 !== 206) { throw "ref 203"; }
score = score + ref203;
var ref204 = 204;
ref204 = ref204 + ordinaryReference;
if (ref204 !== 207) { throw "ref 204"; }
score = score + ref204;
yield = yield + 1;
yield = yield - 1;
if (yield !== 13) { throw "yield stable 204"; }
var ref205 = 205;
ref205 = ref205 + ordinaryReference;
if (ref205 !== 208) { throw "ref 205"; }
score = score + ref205;
var ref206 = 206;
ref206 = ref206 + ordinaryReference;
if (ref206 !== 209) { throw "ref 206"; }
score = score + ref206;
var ref207 = 207;
ref207 = ref207 + ordinaryReference;
if (ref207 !== 210) { throw "ref 207"; }
score = score + ref207;
await = await + 2;
await = await - 2;
if (await !== 33) { throw "await stable 207"; }
var ref208 = 208;
ref208 = ref208 + ordinaryReference;
if (ref208 !== 211) { throw "ref 208"; }
score = score + ref208;
var ref209 = 209;
ref209 = ref209 + ordinaryReference;
if (ref209 !== 212) { throw "ref 209"; }
score = score + ref209;
var ref210 = 210;
ref210 = ref210 + ordinaryReference;
if (ref210 !== 213) { throw "ref 210"; }
score = score + ref210;
var ref211 = 211;
ref211 = ref211 + ordinaryReference;
if (ref211 !== 214) { throw "ref 211"; }
score = score + ref211;
var ref212 = 212;
ref212 = ref212 + ordinaryReference;
if (ref212 !== 215) { throw "ref 212"; }
score = score + ref212;
var ref213 = 213;
ref213 = ref213 + ordinaryReference;
if (ref213 !== 216) { throw "ref 213"; }
score = score + ref213;
var ref214 = 214;
ref214 = ref214 + ordinaryReference;
if (ref214 !== 217) { throw "ref 214"; }
score = score + ref214;
var ref215 = 215;
ref215 = ref215 + ordinaryReference;
if (ref215 !== 218) { throw "ref 215"; }
score = score + ref215;
var ref216 = 216;
ref216 = ref216 + ordinaryReference;
if (ref216 !== 219) { throw "ref 216"; }
score = score + ref216;
var ref217 = 217;
ref217 = ref217 + ordinaryReference;
if (ref217 !== 220) { throw "ref 217"; }
score = score + ref217;
var ref218 = 218;
ref218 = ref218 + ordinaryReference;
if (ref218 !== 221) { throw "ref 218"; }
score = score + ref218;
var ref219 = 219;
ref219 = ref219 + ordinaryReference;
if (ref219 !== 222) { throw "ref 219"; }
score = score + ref219;
var ref220 = 220;
ref220 = ref220 + ordinaryReference;
if (ref220 !== 223) { throw "ref 220"; }
score = score + ref220;
var ref221 = 221;
ref221 = ref221 + ordinaryReference;
if (ref221 !== 224) { throw "ref 221"; }
score = score + ref221;
yield = yield + 1;
yield = yield - 1;
if (yield !== 13) { throw "yield stable 221"; }
var ref222 = 222;
ref222 = ref222 + ordinaryReference;
if (ref222 !== 225) { throw "ref 222"; }
score = score + ref222;
var ref223 = 223;
ref223 = ref223 + ordinaryReference;
if (ref223 !== 226) { throw "ref 223"; }
score = score + ref223;
var ref224 = 224;
ref224 = ref224 + ordinaryReference;
if (ref224 !== 227) { throw "ref 224"; }
score = score + ref224;
var ref225 = 225;
ref225 = ref225 + ordinaryReference;
if (ref225 !== 228) { throw "ref 225"; }
score = score + ref225;
var ref226 = 226;
ref226 = ref226 + ordinaryReference;
if (ref226 !== 229) { throw "ref 226"; }
score = score + ref226;
var ref227 = 227;
ref227 = ref227 + ordinaryReference;
if (ref227 !== 230) { throw "ref 227"; }
score = score + ref227;
return score;
}
console.log("ok", __ayyRun(), ordinaryReference, yield, await);
