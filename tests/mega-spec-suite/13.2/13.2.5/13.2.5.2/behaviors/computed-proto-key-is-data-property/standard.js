// behavior: computed-proto-key-is-data-property
// expected: pass
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

var protoLiteral0 = { inheritedMarker: 10 };
var literalProtoObj0 = { __proto__: protoLiteral0, ownMarker: 10 + 1 };
check(Object.getPrototypeOf(literalProtoObj0) === protoLiteral0, "literal proto prototype 0");
check(literalProtoObj0.inheritedMarker === 10, "literal proto inherited 0");
var protoComputed0 = { dataMarker: 10 + 2 };
var computedProtoObj0 = { ["__proto__"]: protoComputed0, ownMarker: 10 + 3 };
check(Object.getPrototypeOf(computedProtoObj0) === Object.prototype, "computed proto prototype 0");
check(computedProtoObj0.__proto__ === protoComputed0, "computed proto own data 0");
check(computedProtoObj0.ownMarker === 10 + 3, "computed proto other data 0");
var ignoredProtoObj0 = { __proto__: 10, regular0: 10 + 4 };
check(Object.getPrototypeOf(ignoredProtoObj0) === Object.prototype, "non object proto ignored 0");
check(ignoredProtoObj0.regular0 === 10 + 4, "non object proto regular 0");

var protoLiteral1 = { inheritedMarker: 15 };
var literalProtoObj1 = { __proto__: protoLiteral1, ownMarker: 15 + 1 };
check(Object.getPrototypeOf(literalProtoObj1) === protoLiteral1, "literal proto prototype 1");
check(literalProtoObj1.inheritedMarker === 15, "literal proto inherited 1");
var protoComputed1 = { dataMarker: 15 + 2 };
var computedProtoObj1 = { ["__proto__"]: protoComputed1, ownMarker: 15 + 3 };
check(Object.getPrototypeOf(computedProtoObj1) === Object.prototype, "computed proto prototype 1");
check(computedProtoObj1.__proto__ === protoComputed1, "computed proto own data 1");
check(computedProtoObj1.ownMarker === 15 + 3, "computed proto other data 1");
var ignoredProtoObj1 = { __proto__: 15, regular1: 15 + 4 };
check(Object.getPrototypeOf(ignoredProtoObj1) === Object.prototype, "non object proto ignored 1");
check(ignoredProtoObj1.regular1 === 15 + 4, "non object proto regular 1");

var protoLiteral2 = { inheritedMarker: 20 };
var literalProtoObj2 = { __proto__: protoLiteral2, ownMarker: 20 + 1 };
check(Object.getPrototypeOf(literalProtoObj2) === protoLiteral2, "literal proto prototype 2");
check(literalProtoObj2.inheritedMarker === 20, "literal proto inherited 2");
var protoComputed2 = { dataMarker: 20 + 2 };
var computedProtoObj2 = { ["__proto__"]: protoComputed2, ownMarker: 20 + 3 };
check(Object.getPrototypeOf(computedProtoObj2) === Object.prototype, "computed proto prototype 2");
check(computedProtoObj2.__proto__ === protoComputed2, "computed proto own data 2");
check(computedProtoObj2.ownMarker === 20 + 3, "computed proto other data 2");
var ignoredProtoObj2 = { __proto__: 20, regular2: 20 + 4 };
check(Object.getPrototypeOf(ignoredProtoObj2) === Object.prototype, "non object proto ignored 2");
check(ignoredProtoObj2.regular2 === 20 + 4, "non object proto regular 2");

var protoLiteral3 = { inheritedMarker: 25 };
var literalProtoObj3 = { __proto__: protoLiteral3, ownMarker: 25 + 1 };
check(Object.getPrototypeOf(literalProtoObj3) === protoLiteral3, "literal proto prototype 3");
check(literalProtoObj3.inheritedMarker === 25, "literal proto inherited 3");
var protoComputed3 = { dataMarker: 25 + 2 };
var computedProtoObj3 = { ["__proto__"]: protoComputed3, ownMarker: 25 + 3 };
check(Object.getPrototypeOf(computedProtoObj3) === Object.prototype, "computed proto prototype 3");
check(computedProtoObj3.__proto__ === protoComputed3, "computed proto own data 3");
check(computedProtoObj3.ownMarker === 25 + 3, "computed proto other data 3");
var ignoredProtoObj3 = { __proto__: 25, regular3: 25 + 4 };
check(Object.getPrototypeOf(ignoredProtoObj3) === Object.prototype, "non object proto ignored 3");
check(ignoredProtoObj3.regular3 === 25 + 4, "non object proto regular 3");

var protoLiteral4 = { inheritedMarker: 30 };
var literalProtoObj4 = { __proto__: protoLiteral4, ownMarker: 30 + 1 };
check(Object.getPrototypeOf(literalProtoObj4) === protoLiteral4, "literal proto prototype 4");
check(literalProtoObj4.inheritedMarker === 30, "literal proto inherited 4");
var protoComputed4 = { dataMarker: 30 + 2 };
var computedProtoObj4 = { ["__proto__"]: protoComputed4, ownMarker: 30 + 3 };
check(Object.getPrototypeOf(computedProtoObj4) === Object.prototype, "computed proto prototype 4");
check(computedProtoObj4.__proto__ === protoComputed4, "computed proto own data 4");
check(computedProtoObj4.ownMarker === 30 + 3, "computed proto other data 4");
var ignoredProtoObj4 = { __proto__: 30, regular4: 30 + 4 };
check(Object.getPrototypeOf(ignoredProtoObj4) === Object.prototype, "non object proto ignored 4");
check(ignoredProtoObj4.regular4 === 30 + 4, "non object proto regular 4");

var protoLiteral5 = { inheritedMarker: 35 };
var literalProtoObj5 = { __proto__: protoLiteral5, ownMarker: 35 + 1 };
check(Object.getPrototypeOf(literalProtoObj5) === protoLiteral5, "literal proto prototype 5");
check(literalProtoObj5.inheritedMarker === 35, "literal proto inherited 5");
var protoComputed5 = { dataMarker: 35 + 2 };
var computedProtoObj5 = { ["__proto__"]: protoComputed5, ownMarker: 35 + 3 };
check(Object.getPrototypeOf(computedProtoObj5) === Object.prototype, "computed proto prototype 5");
check(computedProtoObj5.__proto__ === protoComputed5, "computed proto own data 5");
check(computedProtoObj5.ownMarker === 35 + 3, "computed proto other data 5");
var ignoredProtoObj5 = { __proto__: 35, regular5: 35 + 4 };
check(Object.getPrototypeOf(ignoredProtoObj5) === Object.prototype, "non object proto ignored 5");
check(ignoredProtoObj5.regular5 === 35 + 4, "non object proto regular 5");

var protoLiteral6 = { inheritedMarker: 40 };
var literalProtoObj6 = { __proto__: protoLiteral6, ownMarker: 40 + 1 };
check(Object.getPrototypeOf(literalProtoObj6) === protoLiteral6, "literal proto prototype 6");
check(literalProtoObj6.inheritedMarker === 40, "literal proto inherited 6");
var protoComputed6 = { dataMarker: 40 + 2 };
var computedProtoObj6 = { ["__proto__"]: protoComputed6, ownMarker: 40 + 3 };
check(Object.getPrototypeOf(computedProtoObj6) === Object.prototype, "computed proto prototype 6");
check(computedProtoObj6.__proto__ === protoComputed6, "computed proto own data 6");
check(computedProtoObj6.ownMarker === 40 + 3, "computed proto other data 6");
var ignoredProtoObj6 = { __proto__: 40, regular6: 40 + 4 };
check(Object.getPrototypeOf(ignoredProtoObj6) === Object.prototype, "non object proto ignored 6");
check(ignoredProtoObj6.regular6 === 40 + 4, "non object proto regular 6");

var protoLiteral7 = { inheritedMarker: 45 };
var literalProtoObj7 = { __proto__: protoLiteral7, ownMarker: 45 + 1 };
check(Object.getPrototypeOf(literalProtoObj7) === protoLiteral7, "literal proto prototype 7");
check(literalProtoObj7.inheritedMarker === 45, "literal proto inherited 7");
var protoComputed7 = { dataMarker: 45 + 2 };
var computedProtoObj7 = { ["__proto__"]: protoComputed7, ownMarker: 45 + 3 };
check(Object.getPrototypeOf(computedProtoObj7) === Object.prototype, "computed proto prototype 7");
check(computedProtoObj7.__proto__ === protoComputed7, "computed proto own data 7");
check(computedProtoObj7.ownMarker === 45 + 3, "computed proto other data 7");
var ignoredProtoObj7 = { __proto__: 45, regular7: 45 + 4 };
check(Object.getPrototypeOf(ignoredProtoObj7) === Object.prototype, "non object proto ignored 7");
check(ignoredProtoObj7.regular7 === 45 + 4, "non object proto regular 7");

var protoLiteral8 = { inheritedMarker: 50 };
var literalProtoObj8 = { __proto__: protoLiteral8, ownMarker: 50 + 1 };
check(Object.getPrototypeOf(literalProtoObj8) === protoLiteral8, "literal proto prototype 8");
check(literalProtoObj8.inheritedMarker === 50, "literal proto inherited 8");
var protoComputed8 = { dataMarker: 50 + 2 };
var computedProtoObj8 = { ["__proto__"]: protoComputed8, ownMarker: 50 + 3 };
check(Object.getPrototypeOf(computedProtoObj8) === Object.prototype, "computed proto prototype 8");
check(computedProtoObj8.__proto__ === protoComputed8, "computed proto own data 8");
check(computedProtoObj8.ownMarker === 50 + 3, "computed proto other data 8");
var ignoredProtoObj8 = { __proto__: 50, regular8: 50 + 4 };
check(Object.getPrototypeOf(ignoredProtoObj8) === Object.prototype, "non object proto ignored 8");
check(ignoredProtoObj8.regular8 === 50 + 4, "non object proto regular 8");

var protoLiteral9 = { inheritedMarker: 55 };
var literalProtoObj9 = { __proto__: protoLiteral9, ownMarker: 55 + 1 };
check(Object.getPrototypeOf(literalProtoObj9) === protoLiteral9, "literal proto prototype 9");
check(literalProtoObj9.inheritedMarker === 55, "literal proto inherited 9");
var protoComputed9 = { dataMarker: 55 + 2 };
var computedProtoObj9 = { ["__proto__"]: protoComputed9, ownMarker: 55 + 3 };
check(Object.getPrototypeOf(computedProtoObj9) === Object.prototype, "computed proto prototype 9");
check(computedProtoObj9.__proto__ === protoComputed9, "computed proto own data 9");
check(computedProtoObj9.ownMarker === 55 + 3, "computed proto other data 9");
var ignoredProtoObj9 = { __proto__: 55, regular9: 55 + 4 };
check(Object.getPrototypeOf(ignoredProtoObj9) === Object.prototype, "non object proto ignored 9");
check(ignoredProtoObj9.regular9 === 55 + 4, "non object proto regular 9");

check(score > 0, "computed proto score");
