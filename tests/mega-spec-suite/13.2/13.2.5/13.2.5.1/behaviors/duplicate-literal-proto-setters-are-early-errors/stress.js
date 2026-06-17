// behavior: duplicate-literal-proto-setters-are-early-errors
// expected: early-error
// goal: script
// size: stress
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

var validProto90 = { marker: 271 };
var validLiteralProto90 = { __proto__: validProto90, own90: 271 + 1 };
check(Object.getPrototypeOf(validLiteralProto90) === validProto90, "valid literal proto 90");
check(validLiteralProto90.own90 === 271 + 1, "valid literal proto own 90");
var validComputedProto90 = { ["__proto__"]: validProto90, other90: 271 + 2 };
check(Object.getPrototypeOf(validComputedProto90) === Object.prototype, "valid computed proto base 90");
check(validComputedProto90.__proto__ === validProto90, "valid computed proto own 90");

var validProto91 = { marker: 274 };
var validLiteralProto91 = { __proto__: validProto91, own91: 274 + 1 };
check(Object.getPrototypeOf(validLiteralProto91) === validProto91, "valid literal proto 91");
check(validLiteralProto91.own91 === 274 + 1, "valid literal proto own 91");
var validComputedProto91 = { ["__proto__"]: validProto91, other91: 274 + 2 };
check(Object.getPrototypeOf(validComputedProto91) === Object.prototype, "valid computed proto base 91");
check(validComputedProto91.__proto__ === validProto91, "valid computed proto own 91");

var validProto92 = { marker: 277 };
var validLiteralProto92 = { __proto__: validProto92, own92: 277 + 1 };
check(Object.getPrototypeOf(validLiteralProto92) === validProto92, "valid literal proto 92");
check(validLiteralProto92.own92 === 277 + 1, "valid literal proto own 92");
var validComputedProto92 = { ["__proto__"]: validProto92, other92: 277 + 2 };
check(Object.getPrototypeOf(validComputedProto92) === Object.prototype, "valid computed proto base 92");
check(validComputedProto92.__proto__ === validProto92, "valid computed proto own 92");

var validProto93 = { marker: 280 };
var validLiteralProto93 = { __proto__: validProto93, own93: 280 + 1 };
check(Object.getPrototypeOf(validLiteralProto93) === validProto93, "valid literal proto 93");
check(validLiteralProto93.own93 === 280 + 1, "valid literal proto own 93");
var validComputedProto93 = { ["__proto__"]: validProto93, other93: 280 + 2 };
check(Object.getPrototypeOf(validComputedProto93) === Object.prototype, "valid computed proto base 93");
check(validComputedProto93.__proto__ === validProto93, "valid computed proto own 93");

var validProto94 = { marker: 283 };
var validLiteralProto94 = { __proto__: validProto94, own94: 283 + 1 };
check(Object.getPrototypeOf(validLiteralProto94) === validProto94, "valid literal proto 94");
check(validLiteralProto94.own94 === 283 + 1, "valid literal proto own 94");
var validComputedProto94 = { ["__proto__"]: validProto94, other94: 283 + 2 };
check(Object.getPrototypeOf(validComputedProto94) === Object.prototype, "valid computed proto base 94");
check(validComputedProto94.__proto__ === validProto94, "valid computed proto own 94");

var validProto95 = { marker: 286 };
var validLiteralProto95 = { __proto__: validProto95, own95: 286 + 1 };
check(Object.getPrototypeOf(validLiteralProto95) === validProto95, "valid literal proto 95");
check(validLiteralProto95.own95 === 286 + 1, "valid literal proto own 95");
var validComputedProto95 = { ["__proto__"]: validProto95, other95: 286 + 2 };
check(Object.getPrototypeOf(validComputedProto95) === Object.prototype, "valid computed proto base 95");
check(validComputedProto95.__proto__ === validProto95, "valid computed proto own 95");

var validProto96 = { marker: 289 };
var validLiteralProto96 = { __proto__: validProto96, own96: 289 + 1 };
check(Object.getPrototypeOf(validLiteralProto96) === validProto96, "valid literal proto 96");
check(validLiteralProto96.own96 === 289 + 1, "valid literal proto own 96");
var validComputedProto96 = { ["__proto__"]: validProto96, other96: 289 + 2 };
check(Object.getPrototypeOf(validComputedProto96) === Object.prototype, "valid computed proto base 96");
check(validComputedProto96.__proto__ === validProto96, "valid computed proto own 96");

var validProto97 = { marker: 292 };
var validLiteralProto97 = { __proto__: validProto97, own97: 292 + 1 };
check(Object.getPrototypeOf(validLiteralProto97) === validProto97, "valid literal proto 97");
check(validLiteralProto97.own97 === 292 + 1, "valid literal proto own 97");
var validComputedProto97 = { ["__proto__"]: validProto97, other97: 292 + 2 };
check(Object.getPrototypeOf(validComputedProto97) === Object.prototype, "valid computed proto base 97");
check(validComputedProto97.__proto__ === validProto97, "valid computed proto own 97");

var validProto98 = { marker: 295 };
var validLiteralProto98 = { __proto__: validProto98, own98: 295 + 1 };
check(Object.getPrototypeOf(validLiteralProto98) === validProto98, "valid literal proto 98");
check(validLiteralProto98.own98 === 295 + 1, "valid literal proto own 98");
var validComputedProto98 = { ["__proto__"]: validProto98, other98: 295 + 2 };
check(Object.getPrototypeOf(validComputedProto98) === Object.prototype, "valid computed proto base 98");
check(validComputedProto98.__proto__ === validProto98, "valid computed proto own 98");

var validProto99 = { marker: 298 };
var validLiteralProto99 = { __proto__: validProto99, own99: 298 + 1 };
check(Object.getPrototypeOf(validLiteralProto99) === validProto99, "valid literal proto 99");
check(validLiteralProto99.own99 === 298 + 1, "valid literal proto own 99");
var validComputedProto99 = { ["__proto__"]: validProto99, other99: 298 + 2 };
check(Object.getPrototypeOf(validComputedProto99) === Object.prototype, "valid computed proto base 99");
check(validComputedProto99.__proto__ === validProto99, "valid computed proto own 99");

var validProto100 = { marker: 301 };
var validLiteralProto100 = { __proto__: validProto100, own100: 301 + 1 };
check(Object.getPrototypeOf(validLiteralProto100) === validProto100, "valid literal proto 100");
check(validLiteralProto100.own100 === 301 + 1, "valid literal proto own 100");
var validComputedProto100 = { ["__proto__"]: validProto100, other100: 301 + 2 };
check(Object.getPrototypeOf(validComputedProto100) === Object.prototype, "valid computed proto base 100");
check(validComputedProto100.__proto__ === validProto100, "valid computed proto own 100");

var validProto101 = { marker: 304 };
var validLiteralProto101 = { __proto__: validProto101, own101: 304 + 1 };
check(Object.getPrototypeOf(validLiteralProto101) === validProto101, "valid literal proto 101");
check(validLiteralProto101.own101 === 304 + 1, "valid literal proto own 101");
var validComputedProto101 = { ["__proto__"]: validProto101, other101: 304 + 2 };
check(Object.getPrototypeOf(validComputedProto101) === Object.prototype, "valid computed proto base 101");
check(validComputedProto101.__proto__ === validProto101, "valid computed proto own 101");

var validProto102 = { marker: 307 };
var validLiteralProto102 = { __proto__: validProto102, own102: 307 + 1 };
check(Object.getPrototypeOf(validLiteralProto102) === validProto102, "valid literal proto 102");
check(validLiteralProto102.own102 === 307 + 1, "valid literal proto own 102");
var validComputedProto102 = { ["__proto__"]: validProto102, other102: 307 + 2 };
check(Object.getPrototypeOf(validComputedProto102) === Object.prototype, "valid computed proto base 102");
check(validComputedProto102.__proto__ === validProto102, "valid computed proto own 102");

var validProto103 = { marker: 310 };
var validLiteralProto103 = { __proto__: validProto103, own103: 310 + 1 };
check(Object.getPrototypeOf(validLiteralProto103) === validProto103, "valid literal proto 103");
check(validLiteralProto103.own103 === 310 + 1, "valid literal proto own 103");
var validComputedProto103 = { ["__proto__"]: validProto103, other103: 310 + 2 };
check(Object.getPrototypeOf(validComputedProto103) === Object.prototype, "valid computed proto base 103");
check(validComputedProto103.__proto__ === validProto103, "valid computed proto own 103");

var validProto104 = { marker: 313 };
var validLiteralProto104 = { __proto__: validProto104, own104: 313 + 1 };
check(Object.getPrototypeOf(validLiteralProto104) === validProto104, "valid literal proto 104");
check(validLiteralProto104.own104 === 313 + 1, "valid literal proto own 104");
var validComputedProto104 = { ["__proto__"]: validProto104, other104: 313 + 2 };
check(Object.getPrototypeOf(validComputedProto104) === Object.prototype, "valid computed proto base 104");
check(validComputedProto104.__proto__ === validProto104, "valid computed proto own 104");

var validProto105 = { marker: 316 };
var validLiteralProto105 = { __proto__: validProto105, own105: 316 + 1 };
check(Object.getPrototypeOf(validLiteralProto105) === validProto105, "valid literal proto 105");
check(validLiteralProto105.own105 === 316 + 1, "valid literal proto own 105");
var validComputedProto105 = { ["__proto__"]: validProto105, other105: 316 + 2 };
check(Object.getPrototypeOf(validComputedProto105) === Object.prototype, "valid computed proto base 105");
check(validComputedProto105.__proto__ === validProto105, "valid computed proto own 105");

var validProto106 = { marker: 319 };
var validLiteralProto106 = { __proto__: validProto106, own106: 319 + 1 };
check(Object.getPrototypeOf(validLiteralProto106) === validProto106, "valid literal proto 106");
check(validLiteralProto106.own106 === 319 + 1, "valid literal proto own 106");
var validComputedProto106 = { ["__proto__"]: validProto106, other106: 319 + 2 };
check(Object.getPrototypeOf(validComputedProto106) === Object.prototype, "valid computed proto base 106");
check(validComputedProto106.__proto__ === validProto106, "valid computed proto own 106");

var validProto107 = { marker: 322 };
var validLiteralProto107 = { __proto__: validProto107, own107: 322 + 1 };
check(Object.getPrototypeOf(validLiteralProto107) === validProto107, "valid literal proto 107");
check(validLiteralProto107.own107 === 322 + 1, "valid literal proto own 107");
var validComputedProto107 = { ["__proto__"]: validProto107, other107: 322 + 2 };
check(Object.getPrototypeOf(validComputedProto107) === Object.prototype, "valid computed proto base 107");
check(validComputedProto107.__proto__ === validProto107, "valid computed proto own 107");

var validProto108 = { marker: 325 };
var validLiteralProto108 = { __proto__: validProto108, own108: 325 + 1 };
check(Object.getPrototypeOf(validLiteralProto108) === validProto108, "valid literal proto 108");
check(validLiteralProto108.own108 === 325 + 1, "valid literal proto own 108");
var validComputedProto108 = { ["__proto__"]: validProto108, other108: 325 + 2 };
check(Object.getPrototypeOf(validComputedProto108) === Object.prototype, "valid computed proto base 108");
check(validComputedProto108.__proto__ === validProto108, "valid computed proto own 108");

var validProto109 = { marker: 328 };
var validLiteralProto109 = { __proto__: validProto109, own109: 328 + 1 };
check(Object.getPrototypeOf(validLiteralProto109) === validProto109, "valid literal proto 109");
check(validLiteralProto109.own109 === 328 + 1, "valid literal proto own 109");
var validComputedProto109 = { ["__proto__"]: validProto109, other109: 328 + 2 };
check(Object.getPrototypeOf(validComputedProto109) === Object.prototype, "valid computed proto base 109");
check(validComputedProto109.__proto__ === validProto109, "valid computed proto own 109");

var validProto110 = { marker: 331 };
var validLiteralProto110 = { __proto__: validProto110, own110: 331 + 1 };
check(Object.getPrototypeOf(validLiteralProto110) === validProto110, "valid literal proto 110");
check(validLiteralProto110.own110 === 331 + 1, "valid literal proto own 110");
var validComputedProto110 = { ["__proto__"]: validProto110, other110: 331 + 2 };
check(Object.getPrototypeOf(validComputedProto110) === Object.prototype, "valid computed proto base 110");
check(validComputedProto110.__proto__ === validProto110, "valid computed proto own 110");

var validProto111 = { marker: 334 };
var validLiteralProto111 = { __proto__: validProto111, own111: 334 + 1 };
check(Object.getPrototypeOf(validLiteralProto111) === validProto111, "valid literal proto 111");
check(validLiteralProto111.own111 === 334 + 1, "valid literal proto own 111");
var validComputedProto111 = { ["__proto__"]: validProto111, other111: 334 + 2 };
check(Object.getPrototypeOf(validComputedProto111) === Object.prototype, "valid computed proto base 111");
check(validComputedProto111.__proto__ === validProto111, "valid computed proto own 111");

var validProto112 = { marker: 337 };
var validLiteralProto112 = { __proto__: validProto112, own112: 337 + 1 };
check(Object.getPrototypeOf(validLiteralProto112) === validProto112, "valid literal proto 112");
check(validLiteralProto112.own112 === 337 + 1, "valid literal proto own 112");
var validComputedProto112 = { ["__proto__"]: validProto112, other112: 337 + 2 };
check(Object.getPrototypeOf(validComputedProto112) === Object.prototype, "valid computed proto base 112");
check(validComputedProto112.__proto__ === validProto112, "valid computed proto own 112");

var validProto113 = { marker: 340 };
var validLiteralProto113 = { __proto__: validProto113, own113: 340 + 1 };
check(Object.getPrototypeOf(validLiteralProto113) === validProto113, "valid literal proto 113");
check(validLiteralProto113.own113 === 340 + 1, "valid literal proto own 113");
var validComputedProto113 = { ["__proto__"]: validProto113, other113: 340 + 2 };
check(Object.getPrototypeOf(validComputedProto113) === Object.prototype, "valid computed proto base 113");
check(validComputedProto113.__proto__ === validProto113, "valid computed proto own 113");

var validProto114 = { marker: 343 };
var validLiteralProto114 = { __proto__: validProto114, own114: 343 + 1 };
check(Object.getPrototypeOf(validLiteralProto114) === validProto114, "valid literal proto 114");
check(validLiteralProto114.own114 === 343 + 1, "valid literal proto own 114");
var validComputedProto114 = { ["__proto__"]: validProto114, other114: 343 + 2 };
check(Object.getPrototypeOf(validComputedProto114) === Object.prototype, "valid computed proto base 114");
check(validComputedProto114.__proto__ === validProto114, "valid computed proto own 114");

var validProto115 = { marker: 346 };
var validLiteralProto115 = { __proto__: validProto115, own115: 346 + 1 };
check(Object.getPrototypeOf(validLiteralProto115) === validProto115, "valid literal proto 115");
check(validLiteralProto115.own115 === 346 + 1, "valid literal proto own 115");
var validComputedProto115 = { ["__proto__"]: validProto115, other115: 346 + 2 };
check(Object.getPrototypeOf(validComputedProto115) === Object.prototype, "valid computed proto base 115");
check(validComputedProto115.__proto__ === validProto115, "valid computed proto own 115");

var validProto116 = { marker: 349 };
var validLiteralProto116 = { __proto__: validProto116, own116: 349 + 1 };
check(Object.getPrototypeOf(validLiteralProto116) === validProto116, "valid literal proto 116");
check(validLiteralProto116.own116 === 349 + 1, "valid literal proto own 116");
var validComputedProto116 = { ["__proto__"]: validProto116, other116: 349 + 2 };
check(Object.getPrototypeOf(validComputedProto116) === Object.prototype, "valid computed proto base 116");
check(validComputedProto116.__proto__ === validProto116, "valid computed proto own 116");

var validProto117 = { marker: 352 };
var validLiteralProto117 = { __proto__: validProto117, own117: 352 + 1 };
check(Object.getPrototypeOf(validLiteralProto117) === validProto117, "valid literal proto 117");
check(validLiteralProto117.own117 === 352 + 1, "valid literal proto own 117");
var validComputedProto117 = { ["__proto__"]: validProto117, other117: 352 + 2 };
check(Object.getPrototypeOf(validComputedProto117) === Object.prototype, "valid computed proto base 117");
check(validComputedProto117.__proto__ === validProto117, "valid computed proto own 117");

var validProto118 = { marker: 355 };
var validLiteralProto118 = { __proto__: validProto118, own118: 355 + 1 };
check(Object.getPrototypeOf(validLiteralProto118) === validProto118, "valid literal proto 118");
check(validLiteralProto118.own118 === 355 + 1, "valid literal proto own 118");
var validComputedProto118 = { ["__proto__"]: validProto118, other118: 355 + 2 };
check(Object.getPrototypeOf(validComputedProto118) === Object.prototype, "valid computed proto base 118");
check(validComputedProto118.__proto__ === validProto118, "valid computed proto own 118");

var validProto119 = { marker: 358 };
var validLiteralProto119 = { __proto__: validProto119, own119: 358 + 1 };
check(Object.getPrototypeOf(validLiteralProto119) === validProto119, "valid literal proto 119");
check(validLiteralProto119.own119 === 358 + 1, "valid literal proto own 119");
var validComputedProto119 = { ["__proto__"]: validProto119, other119: 358 + 2 };
check(Object.getPrototypeOf(validComputedProto119) === Object.prototype, "valid computed proto base 119");
check(validComputedProto119.__proto__ === validProto119, "valid computed proto own 119");

var validProto120 = { marker: 361 };
var validLiteralProto120 = { __proto__: validProto120, own120: 361 + 1 };
check(Object.getPrototypeOf(validLiteralProto120) === validProto120, "valid literal proto 120");
check(validLiteralProto120.own120 === 361 + 1, "valid literal proto own 120");
var validComputedProto120 = { ["__proto__"]: validProto120, other120: 361 + 2 };
check(Object.getPrototypeOf(validComputedProto120) === Object.prototype, "valid computed proto base 120");
check(validComputedProto120.__proto__ === validProto120, "valid computed proto own 120");

var validProto121 = { marker: 364 };
var validLiteralProto121 = { __proto__: validProto121, own121: 364 + 1 };
check(Object.getPrototypeOf(validLiteralProto121) === validProto121, "valid literal proto 121");
check(validLiteralProto121.own121 === 364 + 1, "valid literal proto own 121");
var validComputedProto121 = { ["__proto__"]: validProto121, other121: 364 + 2 };
check(Object.getPrototypeOf(validComputedProto121) === Object.prototype, "valid computed proto base 121");
check(validComputedProto121.__proto__ === validProto121, "valid computed proto own 121");

var validProto122 = { marker: 367 };
var validLiteralProto122 = { __proto__: validProto122, own122: 367 + 1 };
check(Object.getPrototypeOf(validLiteralProto122) === validProto122, "valid literal proto 122");
check(validLiteralProto122.own122 === 367 + 1, "valid literal proto own 122");
var validComputedProto122 = { ["__proto__"]: validProto122, other122: 367 + 2 };
check(Object.getPrototypeOf(validComputedProto122) === Object.prototype, "valid computed proto base 122");
check(validComputedProto122.__proto__ === validProto122, "valid computed proto own 122");

var validProto123 = { marker: 370 };
var validLiteralProto123 = { __proto__: validProto123, own123: 370 + 1 };
check(Object.getPrototypeOf(validLiteralProto123) === validProto123, "valid literal proto 123");
check(validLiteralProto123.own123 === 370 + 1, "valid literal proto own 123");
var validComputedProto123 = { ["__proto__"]: validProto123, other123: 370 + 2 };
check(Object.getPrototypeOf(validComputedProto123) === Object.prototype, "valid computed proto base 123");
check(validComputedProto123.__proto__ === validProto123, "valid computed proto own 123");

var validProto124 = { marker: 373 };
var validLiteralProto124 = { __proto__: validProto124, own124: 373 + 1 };
check(Object.getPrototypeOf(validLiteralProto124) === validProto124, "valid literal proto 124");
check(validLiteralProto124.own124 === 373 + 1, "valid literal proto own 124");
var validComputedProto124 = { ["__proto__"]: validProto124, other124: 373 + 2 };
check(Object.getPrototypeOf(validComputedProto124) === Object.prototype, "valid computed proto base 124");
check(validComputedProto124.__proto__ === validProto124, "valid computed proto own 124");

var validProto125 = { marker: 376 };
var validLiteralProto125 = { __proto__: validProto125, own125: 376 + 1 };
check(Object.getPrototypeOf(validLiteralProto125) === validProto125, "valid literal proto 125");
check(validLiteralProto125.own125 === 376 + 1, "valid literal proto own 125");
var validComputedProto125 = { ["__proto__"]: validProto125, other125: 376 + 2 };
check(Object.getPrototypeOf(validComputedProto125) === Object.prototype, "valid computed proto base 125");
check(validComputedProto125.__proto__ === validProto125, "valid computed proto own 125");

var validProto126 = { marker: 379 };
var validLiteralProto126 = { __proto__: validProto126, own126: 379 + 1 };
check(Object.getPrototypeOf(validLiteralProto126) === validProto126, "valid literal proto 126");
check(validLiteralProto126.own126 === 379 + 1, "valid literal proto own 126");
var validComputedProto126 = { ["__proto__"]: validProto126, other126: 379 + 2 };
check(Object.getPrototypeOf(validComputedProto126) === Object.prototype, "valid computed proto base 126");
check(validComputedProto126.__proto__ === validProto126, "valid computed proto own 126");

var validProto127 = { marker: 382 };
var validLiteralProto127 = { __proto__: validProto127, own127: 382 + 1 };
check(Object.getPrototypeOf(validLiteralProto127) === validProto127, "valid literal proto 127");
check(validLiteralProto127.own127 === 382 + 1, "valid literal proto own 127");
var validComputedProto127 = { ["__proto__"]: validProto127, other127: 382 + 2 };
check(Object.getPrototypeOf(validComputedProto127) === Object.prototype, "valid computed proto base 127");
check(validComputedProto127.__proto__ === validProto127, "valid computed proto own 127");

var validProto128 = { marker: 385 };
var validLiteralProto128 = { __proto__: validProto128, own128: 385 + 1 };
check(Object.getPrototypeOf(validLiteralProto128) === validProto128, "valid literal proto 128");
check(validLiteralProto128.own128 === 385 + 1, "valid literal proto own 128");
var validComputedProto128 = { ["__proto__"]: validProto128, other128: 385 + 2 };
check(Object.getPrototypeOf(validComputedProto128) === Object.prototype, "valid computed proto base 128");
check(validComputedProto128.__proto__ === validProto128, "valid computed proto own 128");

var validProto129 = { marker: 388 };
var validLiteralProto129 = { __proto__: validProto129, own129: 388 + 1 };
check(Object.getPrototypeOf(validLiteralProto129) === validProto129, "valid literal proto 129");
check(validLiteralProto129.own129 === 388 + 1, "valid literal proto own 129");
var validComputedProto129 = { ["__proto__"]: validProto129, other129: 388 + 2 };
check(Object.getPrototypeOf(validComputedProto129) === Object.prototype, "valid computed proto base 129");
check(validComputedProto129.__proto__ === validProto129, "valid computed proto own 129");

var validProto130 = { marker: 391 };
var validLiteralProto130 = { __proto__: validProto130, own130: 391 + 1 };
check(Object.getPrototypeOf(validLiteralProto130) === validProto130, "valid literal proto 130");
check(validLiteralProto130.own130 === 391 + 1, "valid literal proto own 130");
var validComputedProto130 = { ["__proto__"]: validProto130, other130: 391 + 2 };
check(Object.getPrototypeOf(validComputedProto130) === Object.prototype, "valid computed proto base 130");
check(validComputedProto130.__proto__ === validProto130, "valid computed proto own 130");

var validProto131 = { marker: 394 };
var validLiteralProto131 = { __proto__: validProto131, own131: 394 + 1 };
check(Object.getPrototypeOf(validLiteralProto131) === validProto131, "valid literal proto 131");
check(validLiteralProto131.own131 === 394 + 1, "valid literal proto own 131");
var validComputedProto131 = { ["__proto__"]: validProto131, other131: 394 + 2 };
check(Object.getPrototypeOf(validComputedProto131) === Object.prototype, "valid computed proto base 131");
check(validComputedProto131.__proto__ === validProto131, "valid computed proto own 131");

var validProto132 = { marker: 397 };
var validLiteralProto132 = { __proto__: validProto132, own132: 397 + 1 };
check(Object.getPrototypeOf(validLiteralProto132) === validProto132, "valid literal proto 132");
check(validLiteralProto132.own132 === 397 + 1, "valid literal proto own 132");
var validComputedProto132 = { ["__proto__"]: validProto132, other132: 397 + 2 };
check(Object.getPrototypeOf(validComputedProto132) === Object.prototype, "valid computed proto base 132");
check(validComputedProto132.__proto__ === validProto132, "valid computed proto own 132");

var validProto133 = { marker: 400 };
var validLiteralProto133 = { __proto__: validProto133, own133: 400 + 1 };
check(Object.getPrototypeOf(validLiteralProto133) === validProto133, "valid literal proto 133");
check(validLiteralProto133.own133 === 400 + 1, "valid literal proto own 133");
var validComputedProto133 = { ["__proto__"]: validProto133, other133: 400 + 2 };
check(Object.getPrototypeOf(validComputedProto133) === Object.prototype, "valid computed proto base 133");
check(validComputedProto133.__proto__ === validProto133, "valid computed proto own 133");

var validProto134 = { marker: 403 };
var validLiteralProto134 = { __proto__: validProto134, own134: 403 + 1 };
check(Object.getPrototypeOf(validLiteralProto134) === validProto134, "valid literal proto 134");
check(validLiteralProto134.own134 === 403 + 1, "valid literal proto own 134");
var validComputedProto134 = { ["__proto__"]: validProto134, other134: 403 + 2 };
check(Object.getPrototypeOf(validComputedProto134) === Object.prototype, "valid computed proto base 134");
check(validComputedProto134.__proto__ === validProto134, "valid computed proto own 134");

var validProto135 = { marker: 406 };
var validLiteralProto135 = { __proto__: validProto135, own135: 406 + 1 };
check(Object.getPrototypeOf(validLiteralProto135) === validProto135, "valid literal proto 135");
check(validLiteralProto135.own135 === 406 + 1, "valid literal proto own 135");
var validComputedProto135 = { ["__proto__"]: validProto135, other135: 406 + 2 };
check(Object.getPrototypeOf(validComputedProto135) === Object.prototype, "valid computed proto base 135");
check(validComputedProto135.__proto__ === validProto135, "valid computed proto own 135");

var validProto136 = { marker: 409 };
var validLiteralProto136 = { __proto__: validProto136, own136: 409 + 1 };
check(Object.getPrototypeOf(validLiteralProto136) === validProto136, "valid literal proto 136");
check(validLiteralProto136.own136 === 409 + 1, "valid literal proto own 136");
var validComputedProto136 = { ["__proto__"]: validProto136, other136: 409 + 2 };
check(Object.getPrototypeOf(validComputedProto136) === Object.prototype, "valid computed proto base 136");
check(validComputedProto136.__proto__ === validProto136, "valid computed proto own 136");

var validProto137 = { marker: 412 };
var validLiteralProto137 = { __proto__: validProto137, own137: 412 + 1 };
check(Object.getPrototypeOf(validLiteralProto137) === validProto137, "valid literal proto 137");
check(validLiteralProto137.own137 === 412 + 1, "valid literal proto own 137");
var validComputedProto137 = { ["__proto__"]: validProto137, other137: 412 + 2 };
check(Object.getPrototypeOf(validComputedProto137) === Object.prototype, "valid computed proto base 137");
check(validComputedProto137.__proto__ === validProto137, "valid computed proto own 137");

var validProto138 = { marker: 415 };
var validLiteralProto138 = { __proto__: validProto138, own138: 415 + 1 };
check(Object.getPrototypeOf(validLiteralProto138) === validProto138, "valid literal proto 138");
check(validLiteralProto138.own138 === 415 + 1, "valid literal proto own 138");
var validComputedProto138 = { ["__proto__"]: validProto138, other138: 415 + 2 };
check(Object.getPrototypeOf(validComputedProto138) === Object.prototype, "valid computed proto base 138");
check(validComputedProto138.__proto__ === validProto138, "valid computed proto own 138");

var validProto139 = { marker: 418 };
var validLiteralProto139 = { __proto__: validProto139, own139: 418 + 1 };
check(Object.getPrototypeOf(validLiteralProto139) === validProto139, "valid literal proto 139");
check(validLiteralProto139.own139 === 418 + 1, "valid literal proto own 139");
var validComputedProto139 = { ["__proto__"]: validProto139, other139: 418 + 2 };
check(Object.getPrototypeOf(validComputedProto139) === Object.prototype, "valid computed proto base 139");
check(validComputedProto139.__proto__ === validProto139, "valid computed proto own 139");

var validProto140 = { marker: 421 };
var validLiteralProto140 = { __proto__: validProto140, own140: 421 + 1 };
check(Object.getPrototypeOf(validLiteralProto140) === validProto140, "valid literal proto 140");
check(validLiteralProto140.own140 === 421 + 1, "valid literal proto own 140");
var validComputedProto140 = { ["__proto__"]: validProto140, other140: 421 + 2 };
check(Object.getPrototypeOf(validComputedProto140) === Object.prototype, "valid computed proto base 140");
check(validComputedProto140.__proto__ === validProto140, "valid computed proto own 140");

var validProto141 = { marker: 424 };
var validLiteralProto141 = { __proto__: validProto141, own141: 424 + 1 };
check(Object.getPrototypeOf(validLiteralProto141) === validProto141, "valid literal proto 141");
check(validLiteralProto141.own141 === 424 + 1, "valid literal proto own 141");
var validComputedProto141 = { ["__proto__"]: validProto141, other141: 424 + 2 };
check(Object.getPrototypeOf(validComputedProto141) === Object.prototype, "valid computed proto base 141");
check(validComputedProto141.__proto__ === validProto141, "valid computed proto own 141");

var validProto142 = { marker: 427 };
var validLiteralProto142 = { __proto__: validProto142, own142: 427 + 1 };
check(Object.getPrototypeOf(validLiteralProto142) === validProto142, "valid literal proto 142");
check(validLiteralProto142.own142 === 427 + 1, "valid literal proto own 142");
var validComputedProto142 = { ["__proto__"]: validProto142, other142: 427 + 2 };
check(Object.getPrototypeOf(validComputedProto142) === Object.prototype, "valid computed proto base 142");
check(validComputedProto142.__proto__ === validProto142, "valid computed proto own 142");

var validProto143 = { marker: 430 };
var validLiteralProto143 = { __proto__: validProto143, own143: 430 + 1 };
check(Object.getPrototypeOf(validLiteralProto143) === validProto143, "valid literal proto 143");
check(validLiteralProto143.own143 === 430 + 1, "valid literal proto own 143");
var validComputedProto143 = { ["__proto__"]: validProto143, other143: 430 + 2 };
check(Object.getPrototypeOf(validComputedProto143) === Object.prototype, "valid computed proto base 143");
check(validComputedProto143.__proto__ === validProto143, "valid computed proto own 143");

var validProto144 = { marker: 433 };
var validLiteralProto144 = { __proto__: validProto144, own144: 433 + 1 };
check(Object.getPrototypeOf(validLiteralProto144) === validProto144, "valid literal proto 144");
check(validLiteralProto144.own144 === 433 + 1, "valid literal proto own 144");
var validComputedProto144 = { ["__proto__"]: validProto144, other144: 433 + 2 };
check(Object.getPrototypeOf(validComputedProto144) === Object.prototype, "valid computed proto base 144");
check(validComputedProto144.__proto__ === validProto144, "valid computed proto own 144");

var validProto145 = { marker: 436 };
var validLiteralProto145 = { __proto__: validProto145, own145: 436 + 1 };
check(Object.getPrototypeOf(validLiteralProto145) === validProto145, "valid literal proto 145");
check(validLiteralProto145.own145 === 436 + 1, "valid literal proto own 145");
var validComputedProto145 = { ["__proto__"]: validProto145, other145: 436 + 2 };
check(Object.getPrototypeOf(validComputedProto145) === Object.prototype, "valid computed proto base 145");
check(validComputedProto145.__proto__ === validProto145, "valid computed proto own 145");

var validProto146 = { marker: 439 };
var validLiteralProto146 = { __proto__: validProto146, own146: 439 + 1 };
check(Object.getPrototypeOf(validLiteralProto146) === validProto146, "valid literal proto 146");
check(validLiteralProto146.own146 === 439 + 1, "valid literal proto own 146");
var validComputedProto146 = { ["__proto__"]: validProto146, other146: 439 + 2 };
check(Object.getPrototypeOf(validComputedProto146) === Object.prototype, "valid computed proto base 146");
check(validComputedProto146.__proto__ === validProto146, "valid computed proto own 146");

var validProto147 = { marker: 442 };
var validLiteralProto147 = { __proto__: validProto147, own147: 442 + 1 };
check(Object.getPrototypeOf(validLiteralProto147) === validProto147, "valid literal proto 147");
check(validLiteralProto147.own147 === 442 + 1, "valid literal proto own 147");
var validComputedProto147 = { ["__proto__"]: validProto147, other147: 442 + 2 };
check(Object.getPrototypeOf(validComputedProto147) === Object.prototype, "valid computed proto base 147");
check(validComputedProto147.__proto__ === validProto147, "valid computed proto own 147");

var validProto148 = { marker: 445 };
var validLiteralProto148 = { __proto__: validProto148, own148: 445 + 1 };
check(Object.getPrototypeOf(validLiteralProto148) === validProto148, "valid literal proto 148");
check(validLiteralProto148.own148 === 445 + 1, "valid literal proto own 148");
var validComputedProto148 = { ["__proto__"]: validProto148, other148: 445 + 2 };
check(Object.getPrototypeOf(validComputedProto148) === Object.prototype, "valid computed proto base 148");
check(validComputedProto148.__proto__ === validProto148, "valid computed proto own 148");

var validProto149 = { marker: 448 };
var validLiteralProto149 = { __proto__: validProto149, own149: 448 + 1 };
check(Object.getPrototypeOf(validLiteralProto149) === validProto149, "valid literal proto 149");
check(validLiteralProto149.own149 === 448 + 1, "valid literal proto own 149");
var validComputedProto149 = { ["__proto__"]: validProto149, other149: 448 + 2 };
check(Object.getPrototypeOf(validComputedProto149) === Object.prototype, "valid computed proto base 149");
check(validComputedProto149.__proto__ === validProto149, "valid computed proto own 149");

var validProto150 = { marker: 451 };
var validLiteralProto150 = { __proto__: validProto150, own150: 451 + 1 };
check(Object.getPrototypeOf(validLiteralProto150) === validProto150, "valid literal proto 150");
check(validLiteralProto150.own150 === 451 + 1, "valid literal proto own 150");
var validComputedProto150 = { ["__proto__"]: validProto150, other150: 451 + 2 };
check(Object.getPrototypeOf(validComputedProto150) === Object.prototype, "valid computed proto base 150");
check(validComputedProto150.__proto__ === validProto150, "valid computed proto own 150");

var validProto151 = { marker: 454 };
var validLiteralProto151 = { __proto__: validProto151, own151: 454 + 1 };
check(Object.getPrototypeOf(validLiteralProto151) === validProto151, "valid literal proto 151");
check(validLiteralProto151.own151 === 454 + 1, "valid literal proto own 151");
var validComputedProto151 = { ["__proto__"]: validProto151, other151: 454 + 2 };
check(Object.getPrototypeOf(validComputedProto151) === Object.prototype, "valid computed proto base 151");
check(validComputedProto151.__proto__ === validProto151, "valid computed proto own 151");

var validProto152 = { marker: 457 };
var validLiteralProto152 = { __proto__: validProto152, own152: 457 + 1 };
check(Object.getPrototypeOf(validLiteralProto152) === validProto152, "valid literal proto 152");
check(validLiteralProto152.own152 === 457 + 1, "valid literal proto own 152");
var validComputedProto152 = { ["__proto__"]: validProto152, other152: 457 + 2 };
check(Object.getPrototypeOf(validComputedProto152) === Object.prototype, "valid computed proto base 152");
check(validComputedProto152.__proto__ === validProto152, "valid computed proto own 152");

var validProto153 = { marker: 460 };
var validLiteralProto153 = { __proto__: validProto153, own153: 460 + 1 };
check(Object.getPrototypeOf(validLiteralProto153) === validProto153, "valid literal proto 153");
check(validLiteralProto153.own153 === 460 + 1, "valid literal proto own 153");
var validComputedProto153 = { ["__proto__"]: validProto153, other153: 460 + 2 };
check(Object.getPrototypeOf(validComputedProto153) === Object.prototype, "valid computed proto base 153");
check(validComputedProto153.__proto__ === validProto153, "valid computed proto own 153");

var validProto154 = { marker: 463 };
var validLiteralProto154 = { __proto__: validProto154, own154: 463 + 1 };
check(Object.getPrototypeOf(validLiteralProto154) === validProto154, "valid literal proto 154");
check(validLiteralProto154.own154 === 463 + 1, "valid literal proto own 154");
var validComputedProto154 = { ["__proto__"]: validProto154, other154: 463 + 2 };
check(Object.getPrototypeOf(validComputedProto154) === Object.prototype, "valid computed proto base 154");
check(validComputedProto154.__proto__ === validProto154, "valid computed proto own 154");

var validProto155 = { marker: 466 };
var validLiteralProto155 = { __proto__: validProto155, own155: 466 + 1 };
check(Object.getPrototypeOf(validLiteralProto155) === validProto155, "valid literal proto 155");
check(validLiteralProto155.own155 === 466 + 1, "valid literal proto own 155");
var validComputedProto155 = { ["__proto__"]: validProto155, other155: 466 + 2 };
check(Object.getPrototypeOf(validComputedProto155) === Object.prototype, "valid computed proto base 155");
check(validComputedProto155.__proto__ === validProto155, "valid computed proto own 155");

var validProto156 = { marker: 469 };
var validLiteralProto156 = { __proto__: validProto156, own156: 469 + 1 };
check(Object.getPrototypeOf(validLiteralProto156) === validProto156, "valid literal proto 156");
check(validLiteralProto156.own156 === 469 + 1, "valid literal proto own 156");
var validComputedProto156 = { ["__proto__"]: validProto156, other156: 469 + 2 };
check(Object.getPrototypeOf(validComputedProto156) === Object.prototype, "valid computed proto base 156");
check(validComputedProto156.__proto__ === validProto156, "valid computed proto own 156");

var validProto157 = { marker: 472 };
var validLiteralProto157 = { __proto__: validProto157, own157: 472 + 1 };
check(Object.getPrototypeOf(validLiteralProto157) === validProto157, "valid literal proto 157");
check(validLiteralProto157.own157 === 472 + 1, "valid literal proto own 157");
var validComputedProto157 = { ["__proto__"]: validProto157, other157: 472 + 2 };
check(Object.getPrototypeOf(validComputedProto157) === Object.prototype, "valid computed proto base 157");
check(validComputedProto157.__proto__ === validProto157, "valid computed proto own 157");

var validProto158 = { marker: 475 };
var validLiteralProto158 = { __proto__: validProto158, own158: 475 + 1 };
check(Object.getPrototypeOf(validLiteralProto158) === validProto158, "valid literal proto 158");
check(validLiteralProto158.own158 === 475 + 1, "valid literal proto own 158");
var validComputedProto158 = { ["__proto__"]: validProto158, other158: 475 + 2 };
check(Object.getPrototypeOf(validComputedProto158) === Object.prototype, "valid computed proto base 158");
check(validComputedProto158.__proto__ === validProto158, "valid computed proto own 158");

var validProto159 = { marker: 478 };
var validLiteralProto159 = { __proto__: validProto159, own159: 478 + 1 };
check(Object.getPrototypeOf(validLiteralProto159) === validProto159, "valid literal proto 159");
check(validLiteralProto159.own159 === 478 + 1, "valid literal proto own 159");
var validComputedProto159 = { ["__proto__"]: validProto159, other159: 478 + 2 };
check(Object.getPrototypeOf(validComputedProto159) === Object.prototype, "valid computed proto base 159");
check(validComputedProto159.__proto__ === validProto159, "valid computed proto own 159");

var validProto160 = { marker: 481 };
var validLiteralProto160 = { __proto__: validProto160, own160: 481 + 1 };
check(Object.getPrototypeOf(validLiteralProto160) === validProto160, "valid literal proto 160");
check(validLiteralProto160.own160 === 481 + 1, "valid literal proto own 160");
var validComputedProto160 = { ["__proto__"]: validProto160, other160: 481 + 2 };
check(Object.getPrototypeOf(validComputedProto160) === Object.prototype, "valid computed proto base 160");
check(validComputedProto160.__proto__ === validProto160, "valid computed proto own 160");

var validProto161 = { marker: 484 };
var validLiteralProto161 = { __proto__: validProto161, own161: 484 + 1 };
check(Object.getPrototypeOf(validLiteralProto161) === validProto161, "valid literal proto 161");
check(validLiteralProto161.own161 === 484 + 1, "valid literal proto own 161");
var validComputedProto161 = { ["__proto__"]: validProto161, other161: 484 + 2 };
check(Object.getPrototypeOf(validComputedProto161) === Object.prototype, "valid computed proto base 161");
check(validComputedProto161.__proto__ === validProto161, "valid computed proto own 161");

var validProto162 = { marker: 487 };
var validLiteralProto162 = { __proto__: validProto162, own162: 487 + 1 };
check(Object.getPrototypeOf(validLiteralProto162) === validProto162, "valid literal proto 162");
check(validLiteralProto162.own162 === 487 + 1, "valid literal proto own 162");
var validComputedProto162 = { ["__proto__"]: validProto162, other162: 487 + 2 };
check(Object.getPrototypeOf(validComputedProto162) === Object.prototype, "valid computed proto base 162");
check(validComputedProto162.__proto__ === validProto162, "valid computed proto own 162");

var validProto163 = { marker: 490 };
var validLiteralProto163 = { __proto__: validProto163, own163: 490 + 1 };
check(Object.getPrototypeOf(validLiteralProto163) === validProto163, "valid literal proto 163");
check(validLiteralProto163.own163 === 490 + 1, "valid literal proto own 163");
var validComputedProto163 = { ["__proto__"]: validProto163, other163: 490 + 2 };
check(Object.getPrototypeOf(validComputedProto163) === Object.prototype, "valid computed proto base 163");
check(validComputedProto163.__proto__ === validProto163, "valid computed proto own 163");

var validProto164 = { marker: 493 };
var validLiteralProto164 = { __proto__: validProto164, own164: 493 + 1 };
check(Object.getPrototypeOf(validLiteralProto164) === validProto164, "valid literal proto 164");
check(validLiteralProto164.own164 === 493 + 1, "valid literal proto own 164");
var validComputedProto164 = { ["__proto__"]: validProto164, other164: 493 + 2 };
check(Object.getPrototypeOf(validComputedProto164) === Object.prototype, "valid computed proto base 164");
check(validComputedProto164.__proto__ === validProto164, "valid computed proto own 164");

var validProto165 = { marker: 496 };
var validLiteralProto165 = { __proto__: validProto165, own165: 496 + 1 };
check(Object.getPrototypeOf(validLiteralProto165) === validProto165, "valid literal proto 165");
check(validLiteralProto165.own165 === 496 + 1, "valid literal proto own 165");
var validComputedProto165 = { ["__proto__"]: validProto165, other165: 496 + 2 };
check(Object.getPrototypeOf(validComputedProto165) === Object.prototype, "valid computed proto base 165");
check(validComputedProto165.__proto__ === validProto165, "valid computed proto own 165");

var validProto166 = { marker: 499 };
var validLiteralProto166 = { __proto__: validProto166, own166: 499 + 1 };
check(Object.getPrototypeOf(validLiteralProto166) === validProto166, "valid literal proto 166");
check(validLiteralProto166.own166 === 499 + 1, "valid literal proto own 166");
var validComputedProto166 = { ["__proto__"]: validProto166, other166: 499 + 2 };
check(Object.getPrototypeOf(validComputedProto166) === Object.prototype, "valid computed proto base 166");
check(validComputedProto166.__proto__ === validProto166, "valid computed proto own 166");

var validProto167 = { marker: 502 };
var validLiteralProto167 = { __proto__: validProto167, own167: 502 + 1 };
check(Object.getPrototypeOf(validLiteralProto167) === validProto167, "valid literal proto 167");
check(validLiteralProto167.own167 === 502 + 1, "valid literal proto own 167");
var validComputedProto167 = { ["__proto__"]: validProto167, other167: 502 + 2 };
check(Object.getPrototypeOf(validComputedProto167) === Object.prototype, "valid computed proto base 167");
check(validComputedProto167.__proto__ === validProto167, "valid computed proto own 167");

var validProto168 = { marker: 505 };
var validLiteralProto168 = { __proto__: validProto168, own168: 505 + 1 };
check(Object.getPrototypeOf(validLiteralProto168) === validProto168, "valid literal proto 168");
check(validLiteralProto168.own168 === 505 + 1, "valid literal proto own 168");
var validComputedProto168 = { ["__proto__"]: validProto168, other168: 505 + 2 };
check(Object.getPrototypeOf(validComputedProto168) === Object.prototype, "valid computed proto base 168");
check(validComputedProto168.__proto__ === validProto168, "valid computed proto own 168");

var validProto169 = { marker: 508 };
var validLiteralProto169 = { __proto__: validProto169, own169: 508 + 1 };
check(Object.getPrototypeOf(validLiteralProto169) === validProto169, "valid literal proto 169");
check(validLiteralProto169.own169 === 508 + 1, "valid literal proto own 169");
var validComputedProto169 = { ["__proto__"]: validProto169, other169: 508 + 2 };
check(Object.getPrototypeOf(validComputedProto169) === Object.prototype, "valid computed proto base 169");
check(validComputedProto169.__proto__ === validProto169, "valid computed proto own 169");

var validProto170 = { marker: 511 };
var validLiteralProto170 = { __proto__: validProto170, own170: 511 + 1 };
check(Object.getPrototypeOf(validLiteralProto170) === validProto170, "valid literal proto 170");
check(validLiteralProto170.own170 === 511 + 1, "valid literal proto own 170");
var validComputedProto170 = { ["__proto__"]: validProto170, other170: 511 + 2 };
check(Object.getPrototypeOf(validComputedProto170) === Object.prototype, "valid computed proto base 170");
check(validComputedProto170.__proto__ === validProto170, "valid computed proto own 170");

var validProto171 = { marker: 514 };
var validLiteralProto171 = { __proto__: validProto171, own171: 514 + 1 };
check(Object.getPrototypeOf(validLiteralProto171) === validProto171, "valid literal proto 171");
check(validLiteralProto171.own171 === 514 + 1, "valid literal proto own 171");
var validComputedProto171 = { ["__proto__"]: validProto171, other171: 514 + 2 };
check(Object.getPrototypeOf(validComputedProto171) === Object.prototype, "valid computed proto base 171");
check(validComputedProto171.__proto__ === validProto171, "valid computed proto own 171");

var validProto172 = { marker: 517 };
var validLiteralProto172 = { __proto__: validProto172, own172: 517 + 1 };
check(Object.getPrototypeOf(validLiteralProto172) === validProto172, "valid literal proto 172");
check(validLiteralProto172.own172 === 517 + 1, "valid literal proto own 172");
var validComputedProto172 = { ["__proto__"]: validProto172, other172: 517 + 2 };
check(Object.getPrototypeOf(validComputedProto172) === Object.prototype, "valid computed proto base 172");
check(validComputedProto172.__proto__ === validProto172, "valid computed proto own 172");

var validProto173 = { marker: 520 };
var validLiteralProto173 = { __proto__: validProto173, own173: 520 + 1 };
check(Object.getPrototypeOf(validLiteralProto173) === validProto173, "valid literal proto 173");
check(validLiteralProto173.own173 === 520 + 1, "valid literal proto own 173");
var validComputedProto173 = { ["__proto__"]: validProto173, other173: 520 + 2 };
check(Object.getPrototypeOf(validComputedProto173) === Object.prototype, "valid computed proto base 173");
check(validComputedProto173.__proto__ === validProto173, "valid computed proto own 173");

var validProto174 = { marker: 523 };
var validLiteralProto174 = { __proto__: validProto174, own174: 523 + 1 };
check(Object.getPrototypeOf(validLiteralProto174) === validProto174, "valid literal proto 174");
check(validLiteralProto174.own174 === 523 + 1, "valid literal proto own 174");
var validComputedProto174 = { ["__proto__"]: validProto174, other174: 523 + 2 };
check(Object.getPrototypeOf(validComputedProto174) === Object.prototype, "valid computed proto base 174");
check(validComputedProto174.__proto__ === validProto174, "valid computed proto own 174");

var validProto175 = { marker: 526 };
var validLiteralProto175 = { __proto__: validProto175, own175: 526 + 1 };
check(Object.getPrototypeOf(validLiteralProto175) === validProto175, "valid literal proto 175");
check(validLiteralProto175.own175 === 526 + 1, "valid literal proto own 175");
var validComputedProto175 = { ["__proto__"]: validProto175, other175: 526 + 2 };
check(Object.getPrototypeOf(validComputedProto175) === Object.prototype, "valid computed proto base 175");
check(validComputedProto175.__proto__ === validProto175, "valid computed proto own 175");

var validProto176 = { marker: 529 };
var validLiteralProto176 = { __proto__: validProto176, own176: 529 + 1 };
check(Object.getPrototypeOf(validLiteralProto176) === validProto176, "valid literal proto 176");
check(validLiteralProto176.own176 === 529 + 1, "valid literal proto own 176");
var validComputedProto176 = { ["__proto__"]: validProto176, other176: 529 + 2 };
check(Object.getPrototypeOf(validComputedProto176) === Object.prototype, "valid computed proto base 176");
check(validComputedProto176.__proto__ === validProto176, "valid computed proto own 176");

var validProto177 = { marker: 532 };
var validLiteralProto177 = { __proto__: validProto177, own177: 532 + 1 };
check(Object.getPrototypeOf(validLiteralProto177) === validProto177, "valid literal proto 177");
check(validLiteralProto177.own177 === 532 + 1, "valid literal proto own 177");
var validComputedProto177 = { ["__proto__"]: validProto177, other177: 532 + 2 };
check(Object.getPrototypeOf(validComputedProto177) === Object.prototype, "valid computed proto base 177");
check(validComputedProto177.__proto__ === validProto177, "valid computed proto own 177");

var validProto178 = { marker: 535 };
var validLiteralProto178 = { __proto__: validProto178, own178: 535 + 1 };
check(Object.getPrototypeOf(validLiteralProto178) === validProto178, "valid literal proto 178");
check(validLiteralProto178.own178 === 535 + 1, "valid literal proto own 178");
var validComputedProto178 = { ["__proto__"]: validProto178, other178: 535 + 2 };
check(Object.getPrototypeOf(validComputedProto178) === Object.prototype, "valid computed proto base 178");
check(validComputedProto178.__proto__ === validProto178, "valid computed proto own 178");

var validProto179 = { marker: 538 };
var validLiteralProto179 = { __proto__: validProto179, own179: 538 + 1 };
check(Object.getPrototypeOf(validLiteralProto179) === validProto179, "valid literal proto 179");
check(validLiteralProto179.own179 === 538 + 1, "valid literal proto own 179");
var validComputedProto179 = { ["__proto__"]: validProto179, other179: 538 + 2 };
check(Object.getPrototypeOf(validComputedProto179) === Object.prototype, "valid computed proto base 179");
check(validComputedProto179.__proto__ === validProto179, "valid computed proto own 179");

var firstInvalidProto = { marker: 1 };
var secondInvalidProto = { marker: 2 };
var invalidDuplicateProto = { __proto__: firstInvalidProto, "__proto__": secondInvalidProto };
