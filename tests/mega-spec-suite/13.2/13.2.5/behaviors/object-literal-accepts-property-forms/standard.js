// behavior: object-literal-accepts-property-forms
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

function makeSpread(seed) {
return { spreadValue: seed + 6, shared: seed + 7 };
}

var shorthand0 = 1 + 1;
var computedKey0 = "computed_0";
var setterSink0 = 0;
var syntaxObj0 = {
shorthand0,
plain0: 1,
"string-0": 1 + 2,
1000: 1 + 3,
[computedKey0]: 1 + 4,
...makeSpread(1),
method0(delta) { return this.plain0 + delta; },
get slot0() { return this.plain0 + 8; },
set slot0(value) { setterSink0 = value; },
};
check(syntaxObj0.shorthand0 === 1 + 1, "shorthand syntax 0");
check(syntaxObj0.plain0 === 1, "plain syntax 0");
check(syntaxObj0["string-0"] === 1 + 2, "string key syntax 0");
check(syntaxObj0[1000] === 1 + 3, "numeric key syntax 0");
check(syntaxObj0[computedKey0] === 1 + 4, "computed key syntax 0");
check(syntaxObj0.spreadValue === 1 + 6 && syntaxObj0.shared === 1 + 7, "spread syntax 0");
check(syntaxObj0.method0(9) === 1 + 9, "method syntax 0");
check(syntaxObj0.slot0 === 1 + 8, "getter syntax 0");
syntaxObj0.slot0 = 1 + 10;
check(setterSink0 === 1 + 10, "setter syntax 0");

var shorthand1 = 21 + 1;
var computedKey1 = "computed_1";
var setterSink1 = 0;
var syntaxObj1 = {
shorthand1,
plain1: 21,
"string-1": 21 + 2,
1001: 21 + 3,
[computedKey1]: 21 + 4,
...makeSpread(21),
method1(delta) { return this.plain1 + delta; },
get slot1() { return this.plain1 + 8; },
set slot1(value) { setterSink1 = value; },
};
check(syntaxObj1.shorthand1 === 21 + 1, "shorthand syntax 1");
check(syntaxObj1.plain1 === 21, "plain syntax 1");
check(syntaxObj1["string-1"] === 21 + 2, "string key syntax 1");
check(syntaxObj1[1001] === 21 + 3, "numeric key syntax 1");
check(syntaxObj1[computedKey1] === 21 + 4, "computed key syntax 1");
check(syntaxObj1.spreadValue === 21 + 6 && syntaxObj1.shared === 21 + 7, "spread syntax 1");
check(syntaxObj1.method1(9) === 21 + 9, "method syntax 1");
check(syntaxObj1.slot1 === 21 + 8, "getter syntax 1");
syntaxObj1.slot1 = 21 + 10;
check(setterSink1 === 21 + 10, "setter syntax 1");

var shorthand2 = 41 + 1;
var computedKey2 = "computed_2";
var setterSink2 = 0;
var syntaxObj2 = {
shorthand2,
plain2: 41,
"string-2": 41 + 2,
1002: 41 + 3,
[computedKey2]: 41 + 4,
...makeSpread(41),
method2(delta) { return this.plain2 + delta; },
get slot2() { return this.plain2 + 8; },
set slot2(value) { setterSink2 = value; },
};
check(syntaxObj2.shorthand2 === 41 + 1, "shorthand syntax 2");
check(syntaxObj2.plain2 === 41, "plain syntax 2");
check(syntaxObj2["string-2"] === 41 + 2, "string key syntax 2");
check(syntaxObj2[1002] === 41 + 3, "numeric key syntax 2");
check(syntaxObj2[computedKey2] === 41 + 4, "computed key syntax 2");
check(syntaxObj2.spreadValue === 41 + 6 && syntaxObj2.shared === 41 + 7, "spread syntax 2");
check(syntaxObj2.method2(9) === 41 + 9, "method syntax 2");
check(syntaxObj2.slot2 === 41 + 8, "getter syntax 2");
syntaxObj2.slot2 = 41 + 10;
check(setterSink2 === 41 + 10, "setter syntax 2");

var shorthand3 = 61 + 1;
var computedKey3 = "computed_3";
var setterSink3 = 0;
var syntaxObj3 = {
shorthand3,
plain3: 61,
"string-3": 61 + 2,
1003: 61 + 3,
[computedKey3]: 61 + 4,
...makeSpread(61),
method3(delta) { return this.plain3 + delta; },
get slot3() { return this.plain3 + 8; },
set slot3(value) { setterSink3 = value; },
};
check(syntaxObj3.shorthand3 === 61 + 1, "shorthand syntax 3");
check(syntaxObj3.plain3 === 61, "plain syntax 3");
check(syntaxObj3["string-3"] === 61 + 2, "string key syntax 3");
check(syntaxObj3[1003] === 61 + 3, "numeric key syntax 3");
check(syntaxObj3[computedKey3] === 61 + 4, "computed key syntax 3");
check(syntaxObj3.spreadValue === 61 + 6 && syntaxObj3.shared === 61 + 7, "spread syntax 3");
check(syntaxObj3.method3(9) === 61 + 9, "method syntax 3");
check(syntaxObj3.slot3 === 61 + 8, "getter syntax 3");
syntaxObj3.slot3 = 61 + 10;
check(setterSink3 === 61 + 10, "setter syntax 3");

var shorthand4 = 81 + 1;
var computedKey4 = "computed_4";
var setterSink4 = 0;
var syntaxObj4 = {
shorthand4,
plain4: 81,
"string-4": 81 + 2,
1004: 81 + 3,
[computedKey4]: 81 + 4,
...makeSpread(81),
method4(delta) { return this.plain4 + delta; },
get slot4() { return this.plain4 + 8; },
set slot4(value) { setterSink4 = value; },
};
check(syntaxObj4.shorthand4 === 81 + 1, "shorthand syntax 4");
check(syntaxObj4.plain4 === 81, "plain syntax 4");
check(syntaxObj4["string-4"] === 81 + 2, "string key syntax 4");
check(syntaxObj4[1004] === 81 + 3, "numeric key syntax 4");
check(syntaxObj4[computedKey4] === 81 + 4, "computed key syntax 4");
check(syntaxObj4.spreadValue === 81 + 6 && syntaxObj4.shared === 81 + 7, "spread syntax 4");
check(syntaxObj4.method4(9) === 81 + 9, "method syntax 4");
check(syntaxObj4.slot4 === 81 + 8, "getter syntax 4");
syntaxObj4.slot4 = 81 + 10;
check(setterSink4 === 81 + 10, "setter syntax 4");

var shorthand5 = 101 + 1;
var computedKey5 = "computed_5";
var setterSink5 = 0;
var syntaxObj5 = {
shorthand5,
plain5: 101,
"string-5": 101 + 2,
1005: 101 + 3,
[computedKey5]: 101 + 4,
...makeSpread(101),
method5(delta) { return this.plain5 + delta; },
get slot5() { return this.plain5 + 8; },
set slot5(value) { setterSink5 = value; },
};
check(syntaxObj5.shorthand5 === 101 + 1, "shorthand syntax 5");
check(syntaxObj5.plain5 === 101, "plain syntax 5");
check(syntaxObj5["string-5"] === 101 + 2, "string key syntax 5");
check(syntaxObj5[1005] === 101 + 3, "numeric key syntax 5");
check(syntaxObj5[computedKey5] === 101 + 4, "computed key syntax 5");
check(syntaxObj5.spreadValue === 101 + 6 && syntaxObj5.shared === 101 + 7, "spread syntax 5");
check(syntaxObj5.method5(9) === 101 + 9, "method syntax 5");
check(syntaxObj5.slot5 === 101 + 8, "getter syntax 5");
syntaxObj5.slot5 = 101 + 10;
check(setterSink5 === 101 + 10, "setter syntax 5");

var shorthand6 = 121 + 1;
var computedKey6 = "computed_6";
var setterSink6 = 0;
var syntaxObj6 = {
shorthand6,
plain6: 121,
"string-6": 121 + 2,
1006: 121 + 3,
[computedKey6]: 121 + 4,
...makeSpread(121),
method6(delta) { return this.plain6 + delta; },
get slot6() { return this.plain6 + 8; },
set slot6(value) { setterSink6 = value; },
};
check(syntaxObj6.shorthand6 === 121 + 1, "shorthand syntax 6");
check(syntaxObj6.plain6 === 121, "plain syntax 6");
check(syntaxObj6["string-6"] === 121 + 2, "string key syntax 6");
check(syntaxObj6[1006] === 121 + 3, "numeric key syntax 6");
check(syntaxObj6[computedKey6] === 121 + 4, "computed key syntax 6");
check(syntaxObj6.spreadValue === 121 + 6 && syntaxObj6.shared === 121 + 7, "spread syntax 6");
check(syntaxObj6.method6(9) === 121 + 9, "method syntax 6");
check(syntaxObj6.slot6 === 121 + 8, "getter syntax 6");
syntaxObj6.slot6 = 121 + 10;
check(setterSink6 === 121 + 10, "setter syntax 6");

var shorthand7 = 141 + 1;
var computedKey7 = "computed_7";
var setterSink7 = 0;
var syntaxObj7 = {
shorthand7,
plain7: 141,
"string-7": 141 + 2,
1007: 141 + 3,
[computedKey7]: 141 + 4,
...makeSpread(141),
method7(delta) { return this.plain7 + delta; },
get slot7() { return this.plain7 + 8; },
set slot7(value) { setterSink7 = value; },
};
check(syntaxObj7.shorthand7 === 141 + 1, "shorthand syntax 7");
check(syntaxObj7.plain7 === 141, "plain syntax 7");
check(syntaxObj7["string-7"] === 141 + 2, "string key syntax 7");
check(syntaxObj7[1007] === 141 + 3, "numeric key syntax 7");
check(syntaxObj7[computedKey7] === 141 + 4, "computed key syntax 7");
check(syntaxObj7.spreadValue === 141 + 6 && syntaxObj7.shared === 141 + 7, "spread syntax 7");
check(syntaxObj7.method7(9) === 141 + 9, "method syntax 7");
check(syntaxObj7.slot7 === 141 + 8, "getter syntax 7");
syntaxObj7.slot7 = 141 + 10;
check(setterSink7 === 141 + 10, "setter syntax 7");

var shorthand8 = 161 + 1;
var computedKey8 = "computed_8";
var setterSink8 = 0;
var syntaxObj8 = {
shorthand8,
plain8: 161,
"string-8": 161 + 2,
1008: 161 + 3,
[computedKey8]: 161 + 4,
...makeSpread(161),
method8(delta) { return this.plain8 + delta; },
get slot8() { return this.plain8 + 8; },
set slot8(value) { setterSink8 = value; },
};
check(syntaxObj8.shorthand8 === 161 + 1, "shorthand syntax 8");
check(syntaxObj8.plain8 === 161, "plain syntax 8");
check(syntaxObj8["string-8"] === 161 + 2, "string key syntax 8");
check(syntaxObj8[1008] === 161 + 3, "numeric key syntax 8");
check(syntaxObj8[computedKey8] === 161 + 4, "computed key syntax 8");
check(syntaxObj8.spreadValue === 161 + 6 && syntaxObj8.shared === 161 + 7, "spread syntax 8");
check(syntaxObj8.method8(9) === 161 + 9, "method syntax 8");
check(syntaxObj8.slot8 === 161 + 8, "getter syntax 8");
syntaxObj8.slot8 = 161 + 10;
check(setterSink8 === 161 + 10, "setter syntax 8");

var shorthand9 = 181 + 1;
var computedKey9 = "computed_9";
var setterSink9 = 0;
var syntaxObj9 = {
shorthand9,
plain9: 181,
"string-9": 181 + 2,
1009: 181 + 3,
[computedKey9]: 181 + 4,
...makeSpread(181),
method9(delta) { return this.plain9 + delta; },
get slot9() { return this.plain9 + 8; },
set slot9(value) { setterSink9 = value; },
};
check(syntaxObj9.shorthand9 === 181 + 1, "shorthand syntax 9");
check(syntaxObj9.plain9 === 181, "plain syntax 9");
check(syntaxObj9["string-9"] === 181 + 2, "string key syntax 9");
check(syntaxObj9[1009] === 181 + 3, "numeric key syntax 9");
check(syntaxObj9[computedKey9] === 181 + 4, "computed key syntax 9");
check(syntaxObj9.spreadValue === 181 + 6 && syntaxObj9.shared === 181 + 7, "spread syntax 9");
check(syntaxObj9.method9(9) === 181 + 9, "method syntax 9");
check(syntaxObj9.slot9 === 181 + 8, "getter syntax 9");
syntaxObj9.slot9 = 181 + 10;
check(setterSink9 === 181 + 10, "setter syntax 9");

check(score > 0, "object literal syntax score");
