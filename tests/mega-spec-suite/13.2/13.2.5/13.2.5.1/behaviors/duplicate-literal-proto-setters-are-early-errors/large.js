// behavior: duplicate-literal-proto-setters-are-early-errors
// expected: early-error
// goal: script
// size: large
// variant: script.sloppy

var score = 0;
function check(condition, label) {
if (!condition) {
throw label;
}
score = score + 1;
return true;
}

var validProto0 = { marker: 1 };
var validLiteralProto0 = { __proto__: validProto0, own0: 1 + 1 };
check(Object.getPrototypeOf(validLiteralProto0) === validProto0, "valid literal proto 0");
check(validLiteralProto0.own0 === 1 + 1, "valid literal proto own 0");
var validComputedProto0 = { ["__proto__"]: validProto0, other0: 1 + 2 };
check(Object.getPrototypeOf(validComputedProto0) === Object.prototype, "valid computed proto base 0");
check(validComputedProto0.__proto__ === validProto0, "valid computed proto own 0");

var validProto1 = { marker: 4 };
var validLiteralProto1 = { __proto__: validProto1, own1: 4 + 1 };
check(Object.getPrototypeOf(validLiteralProto1) === validProto1, "valid literal proto 1");
check(validLiteralProto1.own1 === 4 + 1, "valid literal proto own 1");
var validComputedProto1 = { ["__proto__"]: validProto1, other1: 4 + 2 };
check(Object.getPrototypeOf(validComputedProto1) === Object.prototype, "valid computed proto base 1");
check(validComputedProto1.__proto__ === validProto1, "valid computed proto own 1");

var validProto2 = { marker: 7 };
var validLiteralProto2 = { __proto__: validProto2, own2: 7 + 1 };
check(Object.getPrototypeOf(validLiteralProto2) === validProto2, "valid literal proto 2");
check(validLiteralProto2.own2 === 7 + 1, "valid literal proto own 2");
var validComputedProto2 = { ["__proto__"]: validProto2, other2: 7 + 2 };
check(Object.getPrototypeOf(validComputedProto2) === Object.prototype, "valid computed proto base 2");
check(validComputedProto2.__proto__ === validProto2, "valid computed proto own 2");

var validProto3 = { marker: 10 };
var validLiteralProto3 = { __proto__: validProto3, own3: 10 + 1 };
check(Object.getPrototypeOf(validLiteralProto3) === validProto3, "valid literal proto 3");
check(validLiteralProto3.own3 === 10 + 1, "valid literal proto own 3");
var validComputedProto3 = { ["__proto__"]: validProto3, other3: 10 + 2 };
check(Object.getPrototypeOf(validComputedProto3) === Object.prototype, "valid computed proto base 3");
check(validComputedProto3.__proto__ === validProto3, "valid computed proto own 3");

var validProto4 = { marker: 13 };
var validLiteralProto4 = { __proto__: validProto4, own4: 13 + 1 };
check(Object.getPrototypeOf(validLiteralProto4) === validProto4, "valid literal proto 4");
check(validLiteralProto4.own4 === 13 + 1, "valid literal proto own 4");
var validComputedProto4 = { ["__proto__"]: validProto4, other4: 13 + 2 };
check(Object.getPrototypeOf(validComputedProto4) === Object.prototype, "valid computed proto base 4");
check(validComputedProto4.__proto__ === validProto4, "valid computed proto own 4");

var validProto5 = { marker: 16 };
var validLiteralProto5 = { __proto__: validProto5, own5: 16 + 1 };
check(Object.getPrototypeOf(validLiteralProto5) === validProto5, "valid literal proto 5");
check(validLiteralProto5.own5 === 16 + 1, "valid literal proto own 5");
var validComputedProto5 = { ["__proto__"]: validProto5, other5: 16 + 2 };
check(Object.getPrototypeOf(validComputedProto5) === Object.prototype, "valid computed proto base 5");
check(validComputedProto5.__proto__ === validProto5, "valid computed proto own 5");

var validProto6 = { marker: 19 };
var validLiteralProto6 = { __proto__: validProto6, own6: 19 + 1 };
check(Object.getPrototypeOf(validLiteralProto6) === validProto6, "valid literal proto 6");
check(validLiteralProto6.own6 === 19 + 1, "valid literal proto own 6");
var validComputedProto6 = { ["__proto__"]: validProto6, other6: 19 + 2 };
check(Object.getPrototypeOf(validComputedProto6) === Object.prototype, "valid computed proto base 6");
check(validComputedProto6.__proto__ === validProto6, "valid computed proto own 6");

var validProto7 = { marker: 22 };
var validLiteralProto7 = { __proto__: validProto7, own7: 22 + 1 };
check(Object.getPrototypeOf(validLiteralProto7) === validProto7, "valid literal proto 7");
check(validLiteralProto7.own7 === 22 + 1, "valid literal proto own 7");
var validComputedProto7 = { ["__proto__"]: validProto7, other7: 22 + 2 };
check(Object.getPrototypeOf(validComputedProto7) === Object.prototype, "valid computed proto base 7");
check(validComputedProto7.__proto__ === validProto7, "valid computed proto own 7");

var validProto8 = { marker: 25 };
var validLiteralProto8 = { __proto__: validProto8, own8: 25 + 1 };
check(Object.getPrototypeOf(validLiteralProto8) === validProto8, "valid literal proto 8");
check(validLiteralProto8.own8 === 25 + 1, "valid literal proto own 8");
var validComputedProto8 = { ["__proto__"]: validProto8, other8: 25 + 2 };
check(Object.getPrototypeOf(validComputedProto8) === Object.prototype, "valid computed proto base 8");
check(validComputedProto8.__proto__ === validProto8, "valid computed proto own 8");

var validProto9 = { marker: 28 };
var validLiteralProto9 = { __proto__: validProto9, own9: 28 + 1 };
check(Object.getPrototypeOf(validLiteralProto9) === validProto9, "valid literal proto 9");
check(validLiteralProto9.own9 === 28 + 1, "valid literal proto own 9");
var validComputedProto9 = { ["__proto__"]: validProto9, other9: 28 + 2 };
check(Object.getPrototypeOf(validComputedProto9) === Object.prototype, "valid computed proto base 9");
check(validComputedProto9.__proto__ === validProto9, "valid computed proto own 9");

var validProto10 = { marker: 31 };
var validLiteralProto10 = { __proto__: validProto10, own10: 31 + 1 };
check(Object.getPrototypeOf(validLiteralProto10) === validProto10, "valid literal proto 10");
check(validLiteralProto10.own10 === 31 + 1, "valid literal proto own 10");
var validComputedProto10 = { ["__proto__"]: validProto10, other10: 31 + 2 };
check(Object.getPrototypeOf(validComputedProto10) === Object.prototype, "valid computed proto base 10");
check(validComputedProto10.__proto__ === validProto10, "valid computed proto own 10");

var validProto11 = { marker: 34 };
var validLiteralProto11 = { __proto__: validProto11, own11: 34 + 1 };
check(Object.getPrototypeOf(validLiteralProto11) === validProto11, "valid literal proto 11");
check(validLiteralProto11.own11 === 34 + 1, "valid literal proto own 11");
var validComputedProto11 = { ["__proto__"]: validProto11, other11: 34 + 2 };
check(Object.getPrototypeOf(validComputedProto11) === Object.prototype, "valid computed proto base 11");
check(validComputedProto11.__proto__ === validProto11, "valid computed proto own 11");

var validProto12 = { marker: 37 };
var validLiteralProto12 = { __proto__: validProto12, own12: 37 + 1 };
check(Object.getPrototypeOf(validLiteralProto12) === validProto12, "valid literal proto 12");
check(validLiteralProto12.own12 === 37 + 1, "valid literal proto own 12");
var validComputedProto12 = { ["__proto__"]: validProto12, other12: 37 + 2 };
check(Object.getPrototypeOf(validComputedProto12) === Object.prototype, "valid computed proto base 12");
check(validComputedProto12.__proto__ === validProto12, "valid computed proto own 12");

var validProto13 = { marker: 40 };
var validLiteralProto13 = { __proto__: validProto13, own13: 40 + 1 };
check(Object.getPrototypeOf(validLiteralProto13) === validProto13, "valid literal proto 13");
check(validLiteralProto13.own13 === 40 + 1, "valid literal proto own 13");
var validComputedProto13 = { ["__proto__"]: validProto13, other13: 40 + 2 };
check(Object.getPrototypeOf(validComputedProto13) === Object.prototype, "valid computed proto base 13");
check(validComputedProto13.__proto__ === validProto13, "valid computed proto own 13");

var validProto14 = { marker: 43 };
var validLiteralProto14 = { __proto__: validProto14, own14: 43 + 1 };
check(Object.getPrototypeOf(validLiteralProto14) === validProto14, "valid literal proto 14");
check(validLiteralProto14.own14 === 43 + 1, "valid literal proto own 14");
var validComputedProto14 = { ["__proto__"]: validProto14, other14: 43 + 2 };
check(Object.getPrototypeOf(validComputedProto14) === Object.prototype, "valid computed proto base 14");
check(validComputedProto14.__proto__ === validProto14, "valid computed proto own 14");

var validProto15 = { marker: 46 };
var validLiteralProto15 = { __proto__: validProto15, own15: 46 + 1 };
check(Object.getPrototypeOf(validLiteralProto15) === validProto15, "valid literal proto 15");
check(validLiteralProto15.own15 === 46 + 1, "valid literal proto own 15");
var validComputedProto15 = { ["__proto__"]: validProto15, other15: 46 + 2 };
check(Object.getPrototypeOf(validComputedProto15) === Object.prototype, "valid computed proto base 15");
check(validComputedProto15.__proto__ === validProto15, "valid computed proto own 15");

var validProto16 = { marker: 49 };
var validLiteralProto16 = { __proto__: validProto16, own16: 49 + 1 };
check(Object.getPrototypeOf(validLiteralProto16) === validProto16, "valid literal proto 16");
check(validLiteralProto16.own16 === 49 + 1, "valid literal proto own 16");
var validComputedProto16 = { ["__proto__"]: validProto16, other16: 49 + 2 };
check(Object.getPrototypeOf(validComputedProto16) === Object.prototype, "valid computed proto base 16");
check(validComputedProto16.__proto__ === validProto16, "valid computed proto own 16");

var validProto17 = { marker: 52 };
var validLiteralProto17 = { __proto__: validProto17, own17: 52 + 1 };
check(Object.getPrototypeOf(validLiteralProto17) === validProto17, "valid literal proto 17");
check(validLiteralProto17.own17 === 52 + 1, "valid literal proto own 17");
var validComputedProto17 = { ["__proto__"]: validProto17, other17: 52 + 2 };
check(Object.getPrototypeOf(validComputedProto17) === Object.prototype, "valid computed proto base 17");
check(validComputedProto17.__proto__ === validProto17, "valid computed proto own 17");

var validProto18 = { marker: 55 };
var validLiteralProto18 = { __proto__: validProto18, own18: 55 + 1 };
check(Object.getPrototypeOf(validLiteralProto18) === validProto18, "valid literal proto 18");
check(validLiteralProto18.own18 === 55 + 1, "valid literal proto own 18");
var validComputedProto18 = { ["__proto__"]: validProto18, other18: 55 + 2 };
check(Object.getPrototypeOf(validComputedProto18) === Object.prototype, "valid computed proto base 18");
check(validComputedProto18.__proto__ === validProto18, "valid computed proto own 18");

var validProto19 = { marker: 58 };
var validLiteralProto19 = { __proto__: validProto19, own19: 58 + 1 };
check(Object.getPrototypeOf(validLiteralProto19) === validProto19, "valid literal proto 19");
check(validLiteralProto19.own19 === 58 + 1, "valid literal proto own 19");
var validComputedProto19 = { ["__proto__"]: validProto19, other19: 58 + 2 };
check(Object.getPrototypeOf(validComputedProto19) === Object.prototype, "valid computed proto base 19");
check(validComputedProto19.__proto__ === validProto19, "valid computed proto own 19");

var validProto20 = { marker: 61 };
var validLiteralProto20 = { __proto__: validProto20, own20: 61 + 1 };
check(Object.getPrototypeOf(validLiteralProto20) === validProto20, "valid literal proto 20");
check(validLiteralProto20.own20 === 61 + 1, "valid literal proto own 20");
var validComputedProto20 = { ["__proto__"]: validProto20, other20: 61 + 2 };
check(Object.getPrototypeOf(validComputedProto20) === Object.prototype, "valid computed proto base 20");
check(validComputedProto20.__proto__ === validProto20, "valid computed proto own 20");

var validProto21 = { marker: 64 };
var validLiteralProto21 = { __proto__: validProto21, own21: 64 + 1 };
check(Object.getPrototypeOf(validLiteralProto21) === validProto21, "valid literal proto 21");
check(validLiteralProto21.own21 === 64 + 1, "valid literal proto own 21");
var validComputedProto21 = { ["__proto__"]: validProto21, other21: 64 + 2 };
check(Object.getPrototypeOf(validComputedProto21) === Object.prototype, "valid computed proto base 21");
check(validComputedProto21.__proto__ === validProto21, "valid computed proto own 21");

var validProto22 = { marker: 67 };
var validLiteralProto22 = { __proto__: validProto22, own22: 67 + 1 };
check(Object.getPrototypeOf(validLiteralProto22) === validProto22, "valid literal proto 22");
check(validLiteralProto22.own22 === 67 + 1, "valid literal proto own 22");
var validComputedProto22 = { ["__proto__"]: validProto22, other22: 67 + 2 };
check(Object.getPrototypeOf(validComputedProto22) === Object.prototype, "valid computed proto base 22");
check(validComputedProto22.__proto__ === validProto22, "valid computed proto own 22");

var validProto23 = { marker: 70 };
var validLiteralProto23 = { __proto__: validProto23, own23: 70 + 1 };
check(Object.getPrototypeOf(validLiteralProto23) === validProto23, "valid literal proto 23");
check(validLiteralProto23.own23 === 70 + 1, "valid literal proto own 23");
var validComputedProto23 = { ["__proto__"]: validProto23, other23: 70 + 2 };
check(Object.getPrototypeOf(validComputedProto23) === Object.prototype, "valid computed proto base 23");
check(validComputedProto23.__proto__ === validProto23, "valid computed proto own 23");

var validProto24 = { marker: 73 };
var validLiteralProto24 = { __proto__: validProto24, own24: 73 + 1 };
check(Object.getPrototypeOf(validLiteralProto24) === validProto24, "valid literal proto 24");
check(validLiteralProto24.own24 === 73 + 1, "valid literal proto own 24");
var validComputedProto24 = { ["__proto__"]: validProto24, other24: 73 + 2 };
check(Object.getPrototypeOf(validComputedProto24) === Object.prototype, "valid computed proto base 24");
check(validComputedProto24.__proto__ === validProto24, "valid computed proto own 24");

var validProto25 = { marker: 76 };
var validLiteralProto25 = { __proto__: validProto25, own25: 76 + 1 };
check(Object.getPrototypeOf(validLiteralProto25) === validProto25, "valid literal proto 25");
check(validLiteralProto25.own25 === 76 + 1, "valid literal proto own 25");
var validComputedProto25 = { ["__proto__"]: validProto25, other25: 76 + 2 };
check(Object.getPrototypeOf(validComputedProto25) === Object.prototype, "valid computed proto base 25");
check(validComputedProto25.__proto__ === validProto25, "valid computed proto own 25");

var validProto26 = { marker: 79 };
var validLiteralProto26 = { __proto__: validProto26, own26: 79 + 1 };
check(Object.getPrototypeOf(validLiteralProto26) === validProto26, "valid literal proto 26");
check(validLiteralProto26.own26 === 79 + 1, "valid literal proto own 26");
var validComputedProto26 = { ["__proto__"]: validProto26, other26: 79 + 2 };
check(Object.getPrototypeOf(validComputedProto26) === Object.prototype, "valid computed proto base 26");
check(validComputedProto26.__proto__ === validProto26, "valid computed proto own 26");

var validProto27 = { marker: 82 };
var validLiteralProto27 = { __proto__: validProto27, own27: 82 + 1 };
check(Object.getPrototypeOf(validLiteralProto27) === validProto27, "valid literal proto 27");
check(validLiteralProto27.own27 === 82 + 1, "valid literal proto own 27");
var validComputedProto27 = { ["__proto__"]: validProto27, other27: 82 + 2 };
check(Object.getPrototypeOf(validComputedProto27) === Object.prototype, "valid computed proto base 27");
check(validComputedProto27.__proto__ === validProto27, "valid computed proto own 27");

var validProto28 = { marker: 85 };
var validLiteralProto28 = { __proto__: validProto28, own28: 85 + 1 };
check(Object.getPrototypeOf(validLiteralProto28) === validProto28, "valid literal proto 28");
check(validLiteralProto28.own28 === 85 + 1, "valid literal proto own 28");
var validComputedProto28 = { ["__proto__"]: validProto28, other28: 85 + 2 };
check(Object.getPrototypeOf(validComputedProto28) === Object.prototype, "valid computed proto base 28");
check(validComputedProto28.__proto__ === validProto28, "valid computed proto own 28");

var validProto29 = { marker: 88 };
var validLiteralProto29 = { __proto__: validProto29, own29: 88 + 1 };
check(Object.getPrototypeOf(validLiteralProto29) === validProto29, "valid literal proto 29");
check(validLiteralProto29.own29 === 88 + 1, "valid literal proto own 29");
var validComputedProto29 = { ["__proto__"]: validProto29, other29: 88 + 2 };
check(Object.getPrototypeOf(validComputedProto29) === Object.prototype, "valid computed proto base 29");
check(validComputedProto29.__proto__ === validProto29, "valid computed proto own 29");

var validProto30 = { marker: 91 };
var validLiteralProto30 = { __proto__: validProto30, own30: 91 + 1 };
check(Object.getPrototypeOf(validLiteralProto30) === validProto30, "valid literal proto 30");
check(validLiteralProto30.own30 === 91 + 1, "valid literal proto own 30");
var validComputedProto30 = { ["__proto__"]: validProto30, other30: 91 + 2 };
check(Object.getPrototypeOf(validComputedProto30) === Object.prototype, "valid computed proto base 30");
check(validComputedProto30.__proto__ === validProto30, "valid computed proto own 30");

var validProto31 = { marker: 94 };
var validLiteralProto31 = { __proto__: validProto31, own31: 94 + 1 };
check(Object.getPrototypeOf(validLiteralProto31) === validProto31, "valid literal proto 31");
check(validLiteralProto31.own31 === 94 + 1, "valid literal proto own 31");
var validComputedProto31 = { ["__proto__"]: validProto31, other31: 94 + 2 };
check(Object.getPrototypeOf(validComputedProto31) === Object.prototype, "valid computed proto base 31");
check(validComputedProto31.__proto__ === validProto31, "valid computed proto own 31");

var validProto32 = { marker: 97 };
var validLiteralProto32 = { __proto__: validProto32, own32: 97 + 1 };
check(Object.getPrototypeOf(validLiteralProto32) === validProto32, "valid literal proto 32");
check(validLiteralProto32.own32 === 97 + 1, "valid literal proto own 32");
var validComputedProto32 = { ["__proto__"]: validProto32, other32: 97 + 2 };
check(Object.getPrototypeOf(validComputedProto32) === Object.prototype, "valid computed proto base 32");
check(validComputedProto32.__proto__ === validProto32, "valid computed proto own 32");

var validProto33 = { marker: 100 };
var validLiteralProto33 = { __proto__: validProto33, own33: 100 + 1 };
check(Object.getPrototypeOf(validLiteralProto33) === validProto33, "valid literal proto 33");
check(validLiteralProto33.own33 === 100 + 1, "valid literal proto own 33");
var validComputedProto33 = { ["__proto__"]: validProto33, other33: 100 + 2 };
check(Object.getPrototypeOf(validComputedProto33) === Object.prototype, "valid computed proto base 33");
check(validComputedProto33.__proto__ === validProto33, "valid computed proto own 33");

var validProto34 = { marker: 103 };
var validLiteralProto34 = { __proto__: validProto34, own34: 103 + 1 };
check(Object.getPrototypeOf(validLiteralProto34) === validProto34, "valid literal proto 34");
check(validLiteralProto34.own34 === 103 + 1, "valid literal proto own 34");
var validComputedProto34 = { ["__proto__"]: validProto34, other34: 103 + 2 };
check(Object.getPrototypeOf(validComputedProto34) === Object.prototype, "valid computed proto base 34");
check(validComputedProto34.__proto__ === validProto34, "valid computed proto own 34");

var validProto35 = { marker: 106 };
var validLiteralProto35 = { __proto__: validProto35, own35: 106 + 1 };
check(Object.getPrototypeOf(validLiteralProto35) === validProto35, "valid literal proto 35");
check(validLiteralProto35.own35 === 106 + 1, "valid literal proto own 35");
var validComputedProto35 = { ["__proto__"]: validProto35, other35: 106 + 2 };
check(Object.getPrototypeOf(validComputedProto35) === Object.prototype, "valid computed proto base 35");
check(validComputedProto35.__proto__ === validProto35, "valid computed proto own 35");

var validProto36 = { marker: 109 };
var validLiteralProto36 = { __proto__: validProto36, own36: 109 + 1 };
check(Object.getPrototypeOf(validLiteralProto36) === validProto36, "valid literal proto 36");
check(validLiteralProto36.own36 === 109 + 1, "valid literal proto own 36");
var validComputedProto36 = { ["__proto__"]: validProto36, other36: 109 + 2 };
check(Object.getPrototypeOf(validComputedProto36) === Object.prototype, "valid computed proto base 36");
check(validComputedProto36.__proto__ === validProto36, "valid computed proto own 36");

var validProto37 = { marker: 112 };
var validLiteralProto37 = { __proto__: validProto37, own37: 112 + 1 };
check(Object.getPrototypeOf(validLiteralProto37) === validProto37, "valid literal proto 37");
check(validLiteralProto37.own37 === 112 + 1, "valid literal proto own 37");
var validComputedProto37 = { ["__proto__"]: validProto37, other37: 112 + 2 };
check(Object.getPrototypeOf(validComputedProto37) === Object.prototype, "valid computed proto base 37");
check(validComputedProto37.__proto__ === validProto37, "valid computed proto own 37");

var validProto38 = { marker: 115 };
var validLiteralProto38 = { __proto__: validProto38, own38: 115 + 1 };
check(Object.getPrototypeOf(validLiteralProto38) === validProto38, "valid literal proto 38");
check(validLiteralProto38.own38 === 115 + 1, "valid literal proto own 38");
var validComputedProto38 = { ["__proto__"]: validProto38, other38: 115 + 2 };
check(Object.getPrototypeOf(validComputedProto38) === Object.prototype, "valid computed proto base 38");
check(validComputedProto38.__proto__ === validProto38, "valid computed proto own 38");

var validProto39 = { marker: 118 };
var validLiteralProto39 = { __proto__: validProto39, own39: 118 + 1 };
check(Object.getPrototypeOf(validLiteralProto39) === validProto39, "valid literal proto 39");
check(validLiteralProto39.own39 === 118 + 1, "valid literal proto own 39");
var validComputedProto39 = { ["__proto__"]: validProto39, other39: 118 + 2 };
check(Object.getPrototypeOf(validComputedProto39) === Object.prototype, "valid computed proto base 39");
check(validComputedProto39.__proto__ === validProto39, "valid computed proto own 39");

var validProto40 = { marker: 121 };
var validLiteralProto40 = { __proto__: validProto40, own40: 121 + 1 };
check(Object.getPrototypeOf(validLiteralProto40) === validProto40, "valid literal proto 40");
check(validLiteralProto40.own40 === 121 + 1, "valid literal proto own 40");
var validComputedProto40 = { ["__proto__"]: validProto40, other40: 121 + 2 };
check(Object.getPrototypeOf(validComputedProto40) === Object.prototype, "valid computed proto base 40");
check(validComputedProto40.__proto__ === validProto40, "valid computed proto own 40");

var validProto41 = { marker: 124 };
var validLiteralProto41 = { __proto__: validProto41, own41: 124 + 1 };
check(Object.getPrototypeOf(validLiteralProto41) === validProto41, "valid literal proto 41");
check(validLiteralProto41.own41 === 124 + 1, "valid literal proto own 41");
var validComputedProto41 = { ["__proto__"]: validProto41, other41: 124 + 2 };
check(Object.getPrototypeOf(validComputedProto41) === Object.prototype, "valid computed proto base 41");
check(validComputedProto41.__proto__ === validProto41, "valid computed proto own 41");

var validProto42 = { marker: 127 };
var validLiteralProto42 = { __proto__: validProto42, own42: 127 + 1 };
check(Object.getPrototypeOf(validLiteralProto42) === validProto42, "valid literal proto 42");
check(validLiteralProto42.own42 === 127 + 1, "valid literal proto own 42");
var validComputedProto42 = { ["__proto__"]: validProto42, other42: 127 + 2 };
check(Object.getPrototypeOf(validComputedProto42) === Object.prototype, "valid computed proto base 42");
check(validComputedProto42.__proto__ === validProto42, "valid computed proto own 42");

var validProto43 = { marker: 130 };
var validLiteralProto43 = { __proto__: validProto43, own43: 130 + 1 };
check(Object.getPrototypeOf(validLiteralProto43) === validProto43, "valid literal proto 43");
check(validLiteralProto43.own43 === 130 + 1, "valid literal proto own 43");
var validComputedProto43 = { ["__proto__"]: validProto43, other43: 130 + 2 };
check(Object.getPrototypeOf(validComputedProto43) === Object.prototype, "valid computed proto base 43");
check(validComputedProto43.__proto__ === validProto43, "valid computed proto own 43");

var validProto44 = { marker: 133 };
var validLiteralProto44 = { __proto__: validProto44, own44: 133 + 1 };
check(Object.getPrototypeOf(validLiteralProto44) === validProto44, "valid literal proto 44");
check(validLiteralProto44.own44 === 133 + 1, "valid literal proto own 44");
var validComputedProto44 = { ["__proto__"]: validProto44, other44: 133 + 2 };
check(Object.getPrototypeOf(validComputedProto44) === Object.prototype, "valid computed proto base 44");
check(validComputedProto44.__proto__ === validProto44, "valid computed proto own 44");

var validProto45 = { marker: 136 };
var validLiteralProto45 = { __proto__: validProto45, own45: 136 + 1 };
check(Object.getPrototypeOf(validLiteralProto45) === validProto45, "valid literal proto 45");
check(validLiteralProto45.own45 === 136 + 1, "valid literal proto own 45");
var validComputedProto45 = { ["__proto__"]: validProto45, other45: 136 + 2 };
check(Object.getPrototypeOf(validComputedProto45) === Object.prototype, "valid computed proto base 45");
check(validComputedProto45.__proto__ === validProto45, "valid computed proto own 45");

var validProto46 = { marker: 139 };
var validLiteralProto46 = { __proto__: validProto46, own46: 139 + 1 };
check(Object.getPrototypeOf(validLiteralProto46) === validProto46, "valid literal proto 46");
check(validLiteralProto46.own46 === 139 + 1, "valid literal proto own 46");
var validComputedProto46 = { ["__proto__"]: validProto46, other46: 139 + 2 };
check(Object.getPrototypeOf(validComputedProto46) === Object.prototype, "valid computed proto base 46");
check(validComputedProto46.__proto__ === validProto46, "valid computed proto own 46");

var validProto47 = { marker: 142 };
var validLiteralProto47 = { __proto__: validProto47, own47: 142 + 1 };
check(Object.getPrototypeOf(validLiteralProto47) === validProto47, "valid literal proto 47");
check(validLiteralProto47.own47 === 142 + 1, "valid literal proto own 47");
var validComputedProto47 = { ["__proto__"]: validProto47, other47: 142 + 2 };
check(Object.getPrototypeOf(validComputedProto47) === Object.prototype, "valid computed proto base 47");
check(validComputedProto47.__proto__ === validProto47, "valid computed proto own 47");

var validProto48 = { marker: 145 };
var validLiteralProto48 = { __proto__: validProto48, own48: 145 + 1 };
check(Object.getPrototypeOf(validLiteralProto48) === validProto48, "valid literal proto 48");
check(validLiteralProto48.own48 === 145 + 1, "valid literal proto own 48");
var validComputedProto48 = { ["__proto__"]: validProto48, other48: 145 + 2 };
check(Object.getPrototypeOf(validComputedProto48) === Object.prototype, "valid computed proto base 48");
check(validComputedProto48.__proto__ === validProto48, "valid computed proto own 48");

var validProto49 = { marker: 148 };
var validLiteralProto49 = { __proto__: validProto49, own49: 148 + 1 };
check(Object.getPrototypeOf(validLiteralProto49) === validProto49, "valid literal proto 49");
check(validLiteralProto49.own49 === 148 + 1, "valid literal proto own 49");
var validComputedProto49 = { ["__proto__"]: validProto49, other49: 148 + 2 };
check(Object.getPrototypeOf(validComputedProto49) === Object.prototype, "valid computed proto base 49");
check(validComputedProto49.__proto__ === validProto49, "valid computed proto own 49");

var validProto50 = { marker: 151 };
var validLiteralProto50 = { __proto__: validProto50, own50: 151 + 1 };
check(Object.getPrototypeOf(validLiteralProto50) === validProto50, "valid literal proto 50");
check(validLiteralProto50.own50 === 151 + 1, "valid literal proto own 50");
var validComputedProto50 = { ["__proto__"]: validProto50, other50: 151 + 2 };
check(Object.getPrototypeOf(validComputedProto50) === Object.prototype, "valid computed proto base 50");
check(validComputedProto50.__proto__ === validProto50, "valid computed proto own 50");

var validProto51 = { marker: 154 };
var validLiteralProto51 = { __proto__: validProto51, own51: 154 + 1 };
check(Object.getPrototypeOf(validLiteralProto51) === validProto51, "valid literal proto 51");
check(validLiteralProto51.own51 === 154 + 1, "valid literal proto own 51");
var validComputedProto51 = { ["__proto__"]: validProto51, other51: 154 + 2 };
check(Object.getPrototypeOf(validComputedProto51) === Object.prototype, "valid computed proto base 51");
check(validComputedProto51.__proto__ === validProto51, "valid computed proto own 51");

var validProto52 = { marker: 157 };
var validLiteralProto52 = { __proto__: validProto52, own52: 157 + 1 };
check(Object.getPrototypeOf(validLiteralProto52) === validProto52, "valid literal proto 52");
check(validLiteralProto52.own52 === 157 + 1, "valid literal proto own 52");
var validComputedProto52 = { ["__proto__"]: validProto52, other52: 157 + 2 };
check(Object.getPrototypeOf(validComputedProto52) === Object.prototype, "valid computed proto base 52");
check(validComputedProto52.__proto__ === validProto52, "valid computed proto own 52");

var validProto53 = { marker: 160 };
var validLiteralProto53 = { __proto__: validProto53, own53: 160 + 1 };
check(Object.getPrototypeOf(validLiteralProto53) === validProto53, "valid literal proto 53");
check(validLiteralProto53.own53 === 160 + 1, "valid literal proto own 53");
var validComputedProto53 = { ["__proto__"]: validProto53, other53: 160 + 2 };
check(Object.getPrototypeOf(validComputedProto53) === Object.prototype, "valid computed proto base 53");
check(validComputedProto53.__proto__ === validProto53, "valid computed proto own 53");

var validProto54 = { marker: 163 };
var validLiteralProto54 = { __proto__: validProto54, own54: 163 + 1 };
check(Object.getPrototypeOf(validLiteralProto54) === validProto54, "valid literal proto 54");
check(validLiteralProto54.own54 === 163 + 1, "valid literal proto own 54");
var validComputedProto54 = { ["__proto__"]: validProto54, other54: 163 + 2 };
check(Object.getPrototypeOf(validComputedProto54) === Object.prototype, "valid computed proto base 54");
check(validComputedProto54.__proto__ === validProto54, "valid computed proto own 54");

var validProto55 = { marker: 166 };
var validLiteralProto55 = { __proto__: validProto55, own55: 166 + 1 };
check(Object.getPrototypeOf(validLiteralProto55) === validProto55, "valid literal proto 55");
check(validLiteralProto55.own55 === 166 + 1, "valid literal proto own 55");
var validComputedProto55 = { ["__proto__"]: validProto55, other55: 166 + 2 };
check(Object.getPrototypeOf(validComputedProto55) === Object.prototype, "valid computed proto base 55");
check(validComputedProto55.__proto__ === validProto55, "valid computed proto own 55");

var validProto56 = { marker: 169 };
var validLiteralProto56 = { __proto__: validProto56, own56: 169 + 1 };
check(Object.getPrototypeOf(validLiteralProto56) === validProto56, "valid literal proto 56");
check(validLiteralProto56.own56 === 169 + 1, "valid literal proto own 56");
var validComputedProto56 = { ["__proto__"]: validProto56, other56: 169 + 2 };
check(Object.getPrototypeOf(validComputedProto56) === Object.prototype, "valid computed proto base 56");
check(validComputedProto56.__proto__ === validProto56, "valid computed proto own 56");

var validProto57 = { marker: 172 };
var validLiteralProto57 = { __proto__: validProto57, own57: 172 + 1 };
check(Object.getPrototypeOf(validLiteralProto57) === validProto57, "valid literal proto 57");
check(validLiteralProto57.own57 === 172 + 1, "valid literal proto own 57");
var validComputedProto57 = { ["__proto__"]: validProto57, other57: 172 + 2 };
check(Object.getPrototypeOf(validComputedProto57) === Object.prototype, "valid computed proto base 57");
check(validComputedProto57.__proto__ === validProto57, "valid computed proto own 57");

var validProto58 = { marker: 175 };
var validLiteralProto58 = { __proto__: validProto58, own58: 175 + 1 };
check(Object.getPrototypeOf(validLiteralProto58) === validProto58, "valid literal proto 58");
check(validLiteralProto58.own58 === 175 + 1, "valid literal proto own 58");
var validComputedProto58 = { ["__proto__"]: validProto58, other58: 175 + 2 };
check(Object.getPrototypeOf(validComputedProto58) === Object.prototype, "valid computed proto base 58");
check(validComputedProto58.__proto__ === validProto58, "valid computed proto own 58");

var validProto59 = { marker: 178 };
var validLiteralProto59 = { __proto__: validProto59, own59: 178 + 1 };
check(Object.getPrototypeOf(validLiteralProto59) === validProto59, "valid literal proto 59");
check(validLiteralProto59.own59 === 178 + 1, "valid literal proto own 59");
var validComputedProto59 = { ["__proto__"]: validProto59, other59: 178 + 2 };
check(Object.getPrototypeOf(validComputedProto59) === Object.prototype, "valid computed proto base 59");
check(validComputedProto59.__proto__ === validProto59, "valid computed proto own 59");

var validProto60 = { marker: 181 };
var validLiteralProto60 = { __proto__: validProto60, own60: 181 + 1 };
check(Object.getPrototypeOf(validLiteralProto60) === validProto60, "valid literal proto 60");
check(validLiteralProto60.own60 === 181 + 1, "valid literal proto own 60");
var validComputedProto60 = { ["__proto__"]: validProto60, other60: 181 + 2 };
check(Object.getPrototypeOf(validComputedProto60) === Object.prototype, "valid computed proto base 60");
check(validComputedProto60.__proto__ === validProto60, "valid computed proto own 60");

var validProto61 = { marker: 184 };
var validLiteralProto61 = { __proto__: validProto61, own61: 184 + 1 };
check(Object.getPrototypeOf(validLiteralProto61) === validProto61, "valid literal proto 61");
check(validLiteralProto61.own61 === 184 + 1, "valid literal proto own 61");
var validComputedProto61 = { ["__proto__"]: validProto61, other61: 184 + 2 };
check(Object.getPrototypeOf(validComputedProto61) === Object.prototype, "valid computed proto base 61");
check(validComputedProto61.__proto__ === validProto61, "valid computed proto own 61");

var validProto62 = { marker: 187 };
var validLiteralProto62 = { __proto__: validProto62, own62: 187 + 1 };
check(Object.getPrototypeOf(validLiteralProto62) === validProto62, "valid literal proto 62");
check(validLiteralProto62.own62 === 187 + 1, "valid literal proto own 62");
var validComputedProto62 = { ["__proto__"]: validProto62, other62: 187 + 2 };
check(Object.getPrototypeOf(validComputedProto62) === Object.prototype, "valid computed proto base 62");
check(validComputedProto62.__proto__ === validProto62, "valid computed proto own 62");

var validProto63 = { marker: 190 };
var validLiteralProto63 = { __proto__: validProto63, own63: 190 + 1 };
check(Object.getPrototypeOf(validLiteralProto63) === validProto63, "valid literal proto 63");
check(validLiteralProto63.own63 === 190 + 1, "valid literal proto own 63");
var validComputedProto63 = { ["__proto__"]: validProto63, other63: 190 + 2 };
check(Object.getPrototypeOf(validComputedProto63) === Object.prototype, "valid computed proto base 63");
check(validComputedProto63.__proto__ === validProto63, "valid computed proto own 63");

var validProto64 = { marker: 193 };
var validLiteralProto64 = { __proto__: validProto64, own64: 193 + 1 };
check(Object.getPrototypeOf(validLiteralProto64) === validProto64, "valid literal proto 64");
check(validLiteralProto64.own64 === 193 + 1, "valid literal proto own 64");
var validComputedProto64 = { ["__proto__"]: validProto64, other64: 193 + 2 };
check(Object.getPrototypeOf(validComputedProto64) === Object.prototype, "valid computed proto base 64");
check(validComputedProto64.__proto__ === validProto64, "valid computed proto own 64");

var validProto65 = { marker: 196 };
var validLiteralProto65 = { __proto__: validProto65, own65: 196 + 1 };
check(Object.getPrototypeOf(validLiteralProto65) === validProto65, "valid literal proto 65");
check(validLiteralProto65.own65 === 196 + 1, "valid literal proto own 65");
var validComputedProto65 = { ["__proto__"]: validProto65, other65: 196 + 2 };
check(Object.getPrototypeOf(validComputedProto65) === Object.prototype, "valid computed proto base 65");
check(validComputedProto65.__proto__ === validProto65, "valid computed proto own 65");

var validProto66 = { marker: 199 };
var validLiteralProto66 = { __proto__: validProto66, own66: 199 + 1 };
check(Object.getPrototypeOf(validLiteralProto66) === validProto66, "valid literal proto 66");
check(validLiteralProto66.own66 === 199 + 1, "valid literal proto own 66");
var validComputedProto66 = { ["__proto__"]: validProto66, other66: 199 + 2 };
check(Object.getPrototypeOf(validComputedProto66) === Object.prototype, "valid computed proto base 66");
check(validComputedProto66.__proto__ === validProto66, "valid computed proto own 66");

var validProto67 = { marker: 202 };
var validLiteralProto67 = { __proto__: validProto67, own67: 202 + 1 };
check(Object.getPrototypeOf(validLiteralProto67) === validProto67, "valid literal proto 67");
check(validLiteralProto67.own67 === 202 + 1, "valid literal proto own 67");
var validComputedProto67 = { ["__proto__"]: validProto67, other67: 202 + 2 };
check(Object.getPrototypeOf(validComputedProto67) === Object.prototype, "valid computed proto base 67");
check(validComputedProto67.__proto__ === validProto67, "valid computed proto own 67");

var validProto68 = { marker: 205 };
var validLiteralProto68 = { __proto__: validProto68, own68: 205 + 1 };
check(Object.getPrototypeOf(validLiteralProto68) === validProto68, "valid literal proto 68");
check(validLiteralProto68.own68 === 205 + 1, "valid literal proto own 68");
var validComputedProto68 = { ["__proto__"]: validProto68, other68: 205 + 2 };
check(Object.getPrototypeOf(validComputedProto68) === Object.prototype, "valid computed proto base 68");
check(validComputedProto68.__proto__ === validProto68, "valid computed proto own 68");

var validProto69 = { marker: 208 };
var validLiteralProto69 = { __proto__: validProto69, own69: 208 + 1 };
check(Object.getPrototypeOf(validLiteralProto69) === validProto69, "valid literal proto 69");
check(validLiteralProto69.own69 === 208 + 1, "valid literal proto own 69");
var validComputedProto69 = { ["__proto__"]: validProto69, other69: 208 + 2 };
check(Object.getPrototypeOf(validComputedProto69) === Object.prototype, "valid computed proto base 69");
check(validComputedProto69.__proto__ === validProto69, "valid computed proto own 69");

var validProto70 = { marker: 211 };
var validLiteralProto70 = { __proto__: validProto70, own70: 211 + 1 };
check(Object.getPrototypeOf(validLiteralProto70) === validProto70, "valid literal proto 70");
check(validLiteralProto70.own70 === 211 + 1, "valid literal proto own 70");
var validComputedProto70 = { ["__proto__"]: validProto70, other70: 211 + 2 };
check(Object.getPrototypeOf(validComputedProto70) === Object.prototype, "valid computed proto base 70");
check(validComputedProto70.__proto__ === validProto70, "valid computed proto own 70");

var validProto71 = { marker: 214 };
var validLiteralProto71 = { __proto__: validProto71, own71: 214 + 1 };
check(Object.getPrototypeOf(validLiteralProto71) === validProto71, "valid literal proto 71");
check(validLiteralProto71.own71 === 214 + 1, "valid literal proto own 71");
var validComputedProto71 = { ["__proto__"]: validProto71, other71: 214 + 2 };
check(Object.getPrototypeOf(validComputedProto71) === Object.prototype, "valid computed proto base 71");
check(validComputedProto71.__proto__ === validProto71, "valid computed proto own 71");

var validProto72 = { marker: 217 };
var validLiteralProto72 = { __proto__: validProto72, own72: 217 + 1 };
check(Object.getPrototypeOf(validLiteralProto72) === validProto72, "valid literal proto 72");
check(validLiteralProto72.own72 === 217 + 1, "valid literal proto own 72");
var validComputedProto72 = { ["__proto__"]: validProto72, other72: 217 + 2 };
check(Object.getPrototypeOf(validComputedProto72) === Object.prototype, "valid computed proto base 72");
check(validComputedProto72.__proto__ === validProto72, "valid computed proto own 72");

var validProto73 = { marker: 220 };
var validLiteralProto73 = { __proto__: validProto73, own73: 220 + 1 };
check(Object.getPrototypeOf(validLiteralProto73) === validProto73, "valid literal proto 73");
check(validLiteralProto73.own73 === 220 + 1, "valid literal proto own 73");
var validComputedProto73 = { ["__proto__"]: validProto73, other73: 220 + 2 };
check(Object.getPrototypeOf(validComputedProto73) === Object.prototype, "valid computed proto base 73");
check(validComputedProto73.__proto__ === validProto73, "valid computed proto own 73");

var validProto74 = { marker: 223 };
var validLiteralProto74 = { __proto__: validProto74, own74: 223 + 1 };
check(Object.getPrototypeOf(validLiteralProto74) === validProto74, "valid literal proto 74");
check(validLiteralProto74.own74 === 223 + 1, "valid literal proto own 74");
var validComputedProto74 = { ["__proto__"]: validProto74, other74: 223 + 2 };
check(Object.getPrototypeOf(validComputedProto74) === Object.prototype, "valid computed proto base 74");
check(validComputedProto74.__proto__ === validProto74, "valid computed proto own 74");

var validProto75 = { marker: 226 };
var validLiteralProto75 = { __proto__: validProto75, own75: 226 + 1 };
check(Object.getPrototypeOf(validLiteralProto75) === validProto75, "valid literal proto 75");
check(validLiteralProto75.own75 === 226 + 1, "valid literal proto own 75");
var validComputedProto75 = { ["__proto__"]: validProto75, other75: 226 + 2 };
check(Object.getPrototypeOf(validComputedProto75) === Object.prototype, "valid computed proto base 75");
check(validComputedProto75.__proto__ === validProto75, "valid computed proto own 75");

var validProto76 = { marker: 229 };
var validLiteralProto76 = { __proto__: validProto76, own76: 229 + 1 };
check(Object.getPrototypeOf(validLiteralProto76) === validProto76, "valid literal proto 76");
check(validLiteralProto76.own76 === 229 + 1, "valid literal proto own 76");
var validComputedProto76 = { ["__proto__"]: validProto76, other76: 229 + 2 };
check(Object.getPrototypeOf(validComputedProto76) === Object.prototype, "valid computed proto base 76");
check(validComputedProto76.__proto__ === validProto76, "valid computed proto own 76");

var validProto77 = { marker: 232 };
var validLiteralProto77 = { __proto__: validProto77, own77: 232 + 1 };
check(Object.getPrototypeOf(validLiteralProto77) === validProto77, "valid literal proto 77");
check(validLiteralProto77.own77 === 232 + 1, "valid literal proto own 77");
var validComputedProto77 = { ["__proto__"]: validProto77, other77: 232 + 2 };
check(Object.getPrototypeOf(validComputedProto77) === Object.prototype, "valid computed proto base 77");
check(validComputedProto77.__proto__ === validProto77, "valid computed proto own 77");

var validProto78 = { marker: 235 };
var validLiteralProto78 = { __proto__: validProto78, own78: 235 + 1 };
check(Object.getPrototypeOf(validLiteralProto78) === validProto78, "valid literal proto 78");
check(validLiteralProto78.own78 === 235 + 1, "valid literal proto own 78");
var validComputedProto78 = { ["__proto__"]: validProto78, other78: 235 + 2 };
check(Object.getPrototypeOf(validComputedProto78) === Object.prototype, "valid computed proto base 78");
check(validComputedProto78.__proto__ === validProto78, "valid computed proto own 78");

var validProto79 = { marker: 238 };
var validLiteralProto79 = { __proto__: validProto79, own79: 238 + 1 };
check(Object.getPrototypeOf(validLiteralProto79) === validProto79, "valid literal proto 79");
check(validLiteralProto79.own79 === 238 + 1, "valid literal proto own 79");
var validComputedProto79 = { ["__proto__"]: validProto79, other79: 238 + 2 };
check(Object.getPrototypeOf(validComputedProto79) === Object.prototype, "valid computed proto base 79");
check(validComputedProto79.__proto__ === validProto79, "valid computed proto own 79");

var validProto80 = { marker: 241 };
var validLiteralProto80 = { __proto__: validProto80, own80: 241 + 1 };
check(Object.getPrototypeOf(validLiteralProto80) === validProto80, "valid literal proto 80");
check(validLiteralProto80.own80 === 241 + 1, "valid literal proto own 80");
var validComputedProto80 = { ["__proto__"]: validProto80, other80: 241 + 2 };
check(Object.getPrototypeOf(validComputedProto80) === Object.prototype, "valid computed proto base 80");
check(validComputedProto80.__proto__ === validProto80, "valid computed proto own 80");

var validProto81 = { marker: 244 };
var validLiteralProto81 = { __proto__: validProto81, own81: 244 + 1 };
check(Object.getPrototypeOf(validLiteralProto81) === validProto81, "valid literal proto 81");
check(validLiteralProto81.own81 === 244 + 1, "valid literal proto own 81");
var validComputedProto81 = { ["__proto__"]: validProto81, other81: 244 + 2 };
check(Object.getPrototypeOf(validComputedProto81) === Object.prototype, "valid computed proto base 81");
check(validComputedProto81.__proto__ === validProto81, "valid computed proto own 81");

var validProto82 = { marker: 247 };
var validLiteralProto82 = { __proto__: validProto82, own82: 247 + 1 };
check(Object.getPrototypeOf(validLiteralProto82) === validProto82, "valid literal proto 82");
check(validLiteralProto82.own82 === 247 + 1, "valid literal proto own 82");
var validComputedProto82 = { ["__proto__"]: validProto82, other82: 247 + 2 };
check(Object.getPrototypeOf(validComputedProto82) === Object.prototype, "valid computed proto base 82");
check(validComputedProto82.__proto__ === validProto82, "valid computed proto own 82");

var validProto83 = { marker: 250 };
var validLiteralProto83 = { __proto__: validProto83, own83: 250 + 1 };
check(Object.getPrototypeOf(validLiteralProto83) === validProto83, "valid literal proto 83");
check(validLiteralProto83.own83 === 250 + 1, "valid literal proto own 83");
var validComputedProto83 = { ["__proto__"]: validProto83, other83: 250 + 2 };
check(Object.getPrototypeOf(validComputedProto83) === Object.prototype, "valid computed proto base 83");
check(validComputedProto83.__proto__ === validProto83, "valid computed proto own 83");

var validProto84 = { marker: 253 };
var validLiteralProto84 = { __proto__: validProto84, own84: 253 + 1 };
check(Object.getPrototypeOf(validLiteralProto84) === validProto84, "valid literal proto 84");
check(validLiteralProto84.own84 === 253 + 1, "valid literal proto own 84");
var validComputedProto84 = { ["__proto__"]: validProto84, other84: 253 + 2 };
check(Object.getPrototypeOf(validComputedProto84) === Object.prototype, "valid computed proto base 84");
check(validComputedProto84.__proto__ === validProto84, "valid computed proto own 84");

var validProto85 = { marker: 256 };
var validLiteralProto85 = { __proto__: validProto85, own85: 256 + 1 };
check(Object.getPrototypeOf(validLiteralProto85) === validProto85, "valid literal proto 85");
check(validLiteralProto85.own85 === 256 + 1, "valid literal proto own 85");
var validComputedProto85 = { ["__proto__"]: validProto85, other85: 256 + 2 };
check(Object.getPrototypeOf(validComputedProto85) === Object.prototype, "valid computed proto base 85");
check(validComputedProto85.__proto__ === validProto85, "valid computed proto own 85");

var validProto86 = { marker: 259 };
var validLiteralProto86 = { __proto__: validProto86, own86: 259 + 1 };
check(Object.getPrototypeOf(validLiteralProto86) === validProto86, "valid literal proto 86");
check(validLiteralProto86.own86 === 259 + 1, "valid literal proto own 86");
var validComputedProto86 = { ["__proto__"]: validProto86, other86: 259 + 2 };
check(Object.getPrototypeOf(validComputedProto86) === Object.prototype, "valid computed proto base 86");
check(validComputedProto86.__proto__ === validProto86, "valid computed proto own 86");

var validProto87 = { marker: 262 };
var validLiteralProto87 = { __proto__: validProto87, own87: 262 + 1 };
check(Object.getPrototypeOf(validLiteralProto87) === validProto87, "valid literal proto 87");
check(validLiteralProto87.own87 === 262 + 1, "valid literal proto own 87");
var validComputedProto87 = { ["__proto__"]: validProto87, other87: 262 + 2 };
check(Object.getPrototypeOf(validComputedProto87) === Object.prototype, "valid computed proto base 87");
check(validComputedProto87.__proto__ === validProto87, "valid computed proto own 87");

var validProto88 = { marker: 265 };
var validLiteralProto88 = { __proto__: validProto88, own88: 265 + 1 };
check(Object.getPrototypeOf(validLiteralProto88) === validProto88, "valid literal proto 88");
check(validLiteralProto88.own88 === 265 + 1, "valid literal proto own 88");
var validComputedProto88 = { ["__proto__"]: validProto88, other88: 265 + 2 };
check(Object.getPrototypeOf(validComputedProto88) === Object.prototype, "valid computed proto base 88");
check(validComputedProto88.__proto__ === validProto88, "valid computed proto own 88");

var validProto89 = { marker: 268 };
var validLiteralProto89 = { __proto__: validProto89, own89: 268 + 1 };
check(Object.getPrototypeOf(validLiteralProto89) === validProto89, "valid literal proto 89");
check(validLiteralProto89.own89 === 268 + 1, "valid literal proto own 89");
var validComputedProto89 = { ["__proto__"]: validProto89, other89: 268 + 2 };
check(Object.getPrototypeOf(validComputedProto89) === Object.prototype, "valid computed proto base 89");
check(validComputedProto89.__proto__ === validProto89, "valid computed proto own 89");

var firstInvalidProto = { marker: 1 };
var secondInvalidProto = { marker: 2 };
var invalidDuplicateProto = { __proto__: firstInvalidProto, "__proto__": secondInvalidProto };
