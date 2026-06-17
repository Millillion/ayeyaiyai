// behavior: duplicate-literal-proto-setters-are-early-errors
// expected: early-error
// goal: script
// size: standard
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

var firstInvalidProto = { marker: 1 };
var secondInvalidProto = { marker: 2 };
var invalidDuplicateProto = { __proto__: firstInvalidProto, "__proto__": secondInvalidProto };
