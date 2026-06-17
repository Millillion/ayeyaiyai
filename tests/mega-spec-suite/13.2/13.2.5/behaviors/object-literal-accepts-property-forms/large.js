// behavior: object-literal-accepts-property-forms
// expected: pass
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

var shorthand10 = 201 + 1;
var computedKey10 = "computed_10";
var setterSink10 = 0;
var syntaxObj10 = {
shorthand10,
plain10: 201,
"string-10": 201 + 2,
1010: 201 + 3,
[computedKey10]: 201 + 4,
...makeSpread(201),
method10(delta) { return this.plain10 + delta; },
get slot10() { return this.plain10 + 8; },
set slot10(value) { setterSink10 = value; },
};
check(syntaxObj10.shorthand10 === 201 + 1, "shorthand syntax 10");
check(syntaxObj10.plain10 === 201, "plain syntax 10");
check(syntaxObj10["string-10"] === 201 + 2, "string key syntax 10");
check(syntaxObj10[1010] === 201 + 3, "numeric key syntax 10");
check(syntaxObj10[computedKey10] === 201 + 4, "computed key syntax 10");
check(syntaxObj10.spreadValue === 201 + 6 && syntaxObj10.shared === 201 + 7, "spread syntax 10");
check(syntaxObj10.method10(9) === 201 + 9, "method syntax 10");
check(syntaxObj10.slot10 === 201 + 8, "getter syntax 10");
syntaxObj10.slot10 = 201 + 10;
check(setterSink10 === 201 + 10, "setter syntax 10");

var shorthand11 = 221 + 1;
var computedKey11 = "computed_11";
var setterSink11 = 0;
var syntaxObj11 = {
shorthand11,
plain11: 221,
"string-11": 221 + 2,
1011: 221 + 3,
[computedKey11]: 221 + 4,
...makeSpread(221),
method11(delta) { return this.plain11 + delta; },
get slot11() { return this.plain11 + 8; },
set slot11(value) { setterSink11 = value; },
};
check(syntaxObj11.shorthand11 === 221 + 1, "shorthand syntax 11");
check(syntaxObj11.plain11 === 221, "plain syntax 11");
check(syntaxObj11["string-11"] === 221 + 2, "string key syntax 11");
check(syntaxObj11[1011] === 221 + 3, "numeric key syntax 11");
check(syntaxObj11[computedKey11] === 221 + 4, "computed key syntax 11");
check(syntaxObj11.spreadValue === 221 + 6 && syntaxObj11.shared === 221 + 7, "spread syntax 11");
check(syntaxObj11.method11(9) === 221 + 9, "method syntax 11");
check(syntaxObj11.slot11 === 221 + 8, "getter syntax 11");
syntaxObj11.slot11 = 221 + 10;
check(setterSink11 === 221 + 10, "setter syntax 11");

var shorthand12 = 241 + 1;
var computedKey12 = "computed_12";
var setterSink12 = 0;
var syntaxObj12 = {
shorthand12,
plain12: 241,
"string-12": 241 + 2,
1012: 241 + 3,
[computedKey12]: 241 + 4,
...makeSpread(241),
method12(delta) { return this.plain12 + delta; },
get slot12() { return this.plain12 + 8; },
set slot12(value) { setterSink12 = value; },
};
check(syntaxObj12.shorthand12 === 241 + 1, "shorthand syntax 12");
check(syntaxObj12.plain12 === 241, "plain syntax 12");
check(syntaxObj12["string-12"] === 241 + 2, "string key syntax 12");
check(syntaxObj12[1012] === 241 + 3, "numeric key syntax 12");
check(syntaxObj12[computedKey12] === 241 + 4, "computed key syntax 12");
check(syntaxObj12.spreadValue === 241 + 6 && syntaxObj12.shared === 241 + 7, "spread syntax 12");
check(syntaxObj12.method12(9) === 241 + 9, "method syntax 12");
check(syntaxObj12.slot12 === 241 + 8, "getter syntax 12");
syntaxObj12.slot12 = 241 + 10;
check(setterSink12 === 241 + 10, "setter syntax 12");

var shorthand13 = 261 + 1;
var computedKey13 = "computed_13";
var setterSink13 = 0;
var syntaxObj13 = {
shorthand13,
plain13: 261,
"string-13": 261 + 2,
1013: 261 + 3,
[computedKey13]: 261 + 4,
...makeSpread(261),
method13(delta) { return this.plain13 + delta; },
get slot13() { return this.plain13 + 8; },
set slot13(value) { setterSink13 = value; },
};
check(syntaxObj13.shorthand13 === 261 + 1, "shorthand syntax 13");
check(syntaxObj13.plain13 === 261, "plain syntax 13");
check(syntaxObj13["string-13"] === 261 + 2, "string key syntax 13");
check(syntaxObj13[1013] === 261 + 3, "numeric key syntax 13");
check(syntaxObj13[computedKey13] === 261 + 4, "computed key syntax 13");
check(syntaxObj13.spreadValue === 261 + 6 && syntaxObj13.shared === 261 + 7, "spread syntax 13");
check(syntaxObj13.method13(9) === 261 + 9, "method syntax 13");
check(syntaxObj13.slot13 === 261 + 8, "getter syntax 13");
syntaxObj13.slot13 = 261 + 10;
check(setterSink13 === 261 + 10, "setter syntax 13");

var shorthand14 = 281 + 1;
var computedKey14 = "computed_14";
var setterSink14 = 0;
var syntaxObj14 = {
shorthand14,
plain14: 281,
"string-14": 281 + 2,
1014: 281 + 3,
[computedKey14]: 281 + 4,
...makeSpread(281),
method14(delta) { return this.plain14 + delta; },
get slot14() { return this.plain14 + 8; },
set slot14(value) { setterSink14 = value; },
};
check(syntaxObj14.shorthand14 === 281 + 1, "shorthand syntax 14");
check(syntaxObj14.plain14 === 281, "plain syntax 14");
check(syntaxObj14["string-14"] === 281 + 2, "string key syntax 14");
check(syntaxObj14[1014] === 281 + 3, "numeric key syntax 14");
check(syntaxObj14[computedKey14] === 281 + 4, "computed key syntax 14");
check(syntaxObj14.spreadValue === 281 + 6 && syntaxObj14.shared === 281 + 7, "spread syntax 14");
check(syntaxObj14.method14(9) === 281 + 9, "method syntax 14");
check(syntaxObj14.slot14 === 281 + 8, "getter syntax 14");
syntaxObj14.slot14 = 281 + 10;
check(setterSink14 === 281 + 10, "setter syntax 14");

var shorthand15 = 301 + 1;
var computedKey15 = "computed_15";
var setterSink15 = 0;
var syntaxObj15 = {
shorthand15,
plain15: 301,
"string-15": 301 + 2,
1015: 301 + 3,
[computedKey15]: 301 + 4,
...makeSpread(301),
method15(delta) { return this.plain15 + delta; },
get slot15() { return this.plain15 + 8; },
set slot15(value) { setterSink15 = value; },
};
check(syntaxObj15.shorthand15 === 301 + 1, "shorthand syntax 15");
check(syntaxObj15.plain15 === 301, "plain syntax 15");
check(syntaxObj15["string-15"] === 301 + 2, "string key syntax 15");
check(syntaxObj15[1015] === 301 + 3, "numeric key syntax 15");
check(syntaxObj15[computedKey15] === 301 + 4, "computed key syntax 15");
check(syntaxObj15.spreadValue === 301 + 6 && syntaxObj15.shared === 301 + 7, "spread syntax 15");
check(syntaxObj15.method15(9) === 301 + 9, "method syntax 15");
check(syntaxObj15.slot15 === 301 + 8, "getter syntax 15");
syntaxObj15.slot15 = 301 + 10;
check(setterSink15 === 301 + 10, "setter syntax 15");

var shorthand16 = 321 + 1;
var computedKey16 = "computed_16";
var setterSink16 = 0;
var syntaxObj16 = {
shorthand16,
plain16: 321,
"string-16": 321 + 2,
1016: 321 + 3,
[computedKey16]: 321 + 4,
...makeSpread(321),
method16(delta) { return this.plain16 + delta; },
get slot16() { return this.plain16 + 8; },
set slot16(value) { setterSink16 = value; },
};
check(syntaxObj16.shorthand16 === 321 + 1, "shorthand syntax 16");
check(syntaxObj16.plain16 === 321, "plain syntax 16");
check(syntaxObj16["string-16"] === 321 + 2, "string key syntax 16");
check(syntaxObj16[1016] === 321 + 3, "numeric key syntax 16");
check(syntaxObj16[computedKey16] === 321 + 4, "computed key syntax 16");
check(syntaxObj16.spreadValue === 321 + 6 && syntaxObj16.shared === 321 + 7, "spread syntax 16");
check(syntaxObj16.method16(9) === 321 + 9, "method syntax 16");
check(syntaxObj16.slot16 === 321 + 8, "getter syntax 16");
syntaxObj16.slot16 = 321 + 10;
check(setterSink16 === 321 + 10, "setter syntax 16");

var shorthand17 = 341 + 1;
var computedKey17 = "computed_17";
var setterSink17 = 0;
var syntaxObj17 = {
shorthand17,
plain17: 341,
"string-17": 341 + 2,
1017: 341 + 3,
[computedKey17]: 341 + 4,
...makeSpread(341),
method17(delta) { return this.plain17 + delta; },
get slot17() { return this.plain17 + 8; },
set slot17(value) { setterSink17 = value; },
};
check(syntaxObj17.shorthand17 === 341 + 1, "shorthand syntax 17");
check(syntaxObj17.plain17 === 341, "plain syntax 17");
check(syntaxObj17["string-17"] === 341 + 2, "string key syntax 17");
check(syntaxObj17[1017] === 341 + 3, "numeric key syntax 17");
check(syntaxObj17[computedKey17] === 341 + 4, "computed key syntax 17");
check(syntaxObj17.spreadValue === 341 + 6 && syntaxObj17.shared === 341 + 7, "spread syntax 17");
check(syntaxObj17.method17(9) === 341 + 9, "method syntax 17");
check(syntaxObj17.slot17 === 341 + 8, "getter syntax 17");
syntaxObj17.slot17 = 341 + 10;
check(setterSink17 === 341 + 10, "setter syntax 17");

var shorthand18 = 361 + 1;
var computedKey18 = "computed_18";
var setterSink18 = 0;
var syntaxObj18 = {
shorthand18,
plain18: 361,
"string-18": 361 + 2,
1018: 361 + 3,
[computedKey18]: 361 + 4,
...makeSpread(361),
method18(delta) { return this.plain18 + delta; },
get slot18() { return this.plain18 + 8; },
set slot18(value) { setterSink18 = value; },
};
check(syntaxObj18.shorthand18 === 361 + 1, "shorthand syntax 18");
check(syntaxObj18.plain18 === 361, "plain syntax 18");
check(syntaxObj18["string-18"] === 361 + 2, "string key syntax 18");
check(syntaxObj18[1018] === 361 + 3, "numeric key syntax 18");
check(syntaxObj18[computedKey18] === 361 + 4, "computed key syntax 18");
check(syntaxObj18.spreadValue === 361 + 6 && syntaxObj18.shared === 361 + 7, "spread syntax 18");
check(syntaxObj18.method18(9) === 361 + 9, "method syntax 18");
check(syntaxObj18.slot18 === 361 + 8, "getter syntax 18");
syntaxObj18.slot18 = 361 + 10;
check(setterSink18 === 361 + 10, "setter syntax 18");

var shorthand19 = 381 + 1;
var computedKey19 = "computed_19";
var setterSink19 = 0;
var syntaxObj19 = {
shorthand19,
plain19: 381,
"string-19": 381 + 2,
1019: 381 + 3,
[computedKey19]: 381 + 4,
...makeSpread(381),
method19(delta) { return this.plain19 + delta; },
get slot19() { return this.plain19 + 8; },
set slot19(value) { setterSink19 = value; },
};
check(syntaxObj19.shorthand19 === 381 + 1, "shorthand syntax 19");
check(syntaxObj19.plain19 === 381, "plain syntax 19");
check(syntaxObj19["string-19"] === 381 + 2, "string key syntax 19");
check(syntaxObj19[1019] === 381 + 3, "numeric key syntax 19");
check(syntaxObj19[computedKey19] === 381 + 4, "computed key syntax 19");
check(syntaxObj19.spreadValue === 381 + 6 && syntaxObj19.shared === 381 + 7, "spread syntax 19");
check(syntaxObj19.method19(9) === 381 + 9, "method syntax 19");
check(syntaxObj19.slot19 === 381 + 8, "getter syntax 19");
syntaxObj19.slot19 = 381 + 10;
check(setterSink19 === 381 + 10, "setter syntax 19");

var shorthand20 = 401 + 1;
var computedKey20 = "computed_20";
var setterSink20 = 0;
var syntaxObj20 = {
shorthand20,
plain20: 401,
"string-20": 401 + 2,
1020: 401 + 3,
[computedKey20]: 401 + 4,
...makeSpread(401),
method20(delta) { return this.plain20 + delta; },
get slot20() { return this.plain20 + 8; },
set slot20(value) { setterSink20 = value; },
};
check(syntaxObj20.shorthand20 === 401 + 1, "shorthand syntax 20");
check(syntaxObj20.plain20 === 401, "plain syntax 20");
check(syntaxObj20["string-20"] === 401 + 2, "string key syntax 20");
check(syntaxObj20[1020] === 401 + 3, "numeric key syntax 20");
check(syntaxObj20[computedKey20] === 401 + 4, "computed key syntax 20");
check(syntaxObj20.spreadValue === 401 + 6 && syntaxObj20.shared === 401 + 7, "spread syntax 20");
check(syntaxObj20.method20(9) === 401 + 9, "method syntax 20");
check(syntaxObj20.slot20 === 401 + 8, "getter syntax 20");
syntaxObj20.slot20 = 401 + 10;
check(setterSink20 === 401 + 10, "setter syntax 20");

var shorthand21 = 421 + 1;
var computedKey21 = "computed_21";
var setterSink21 = 0;
var syntaxObj21 = {
shorthand21,
plain21: 421,
"string-21": 421 + 2,
1021: 421 + 3,
[computedKey21]: 421 + 4,
...makeSpread(421),
method21(delta) { return this.plain21 + delta; },
get slot21() { return this.plain21 + 8; },
set slot21(value) { setterSink21 = value; },
};
check(syntaxObj21.shorthand21 === 421 + 1, "shorthand syntax 21");
check(syntaxObj21.plain21 === 421, "plain syntax 21");
check(syntaxObj21["string-21"] === 421 + 2, "string key syntax 21");
check(syntaxObj21[1021] === 421 + 3, "numeric key syntax 21");
check(syntaxObj21[computedKey21] === 421 + 4, "computed key syntax 21");
check(syntaxObj21.spreadValue === 421 + 6 && syntaxObj21.shared === 421 + 7, "spread syntax 21");
check(syntaxObj21.method21(9) === 421 + 9, "method syntax 21");
check(syntaxObj21.slot21 === 421 + 8, "getter syntax 21");
syntaxObj21.slot21 = 421 + 10;
check(setterSink21 === 421 + 10, "setter syntax 21");

var shorthand22 = 441 + 1;
var computedKey22 = "computed_22";
var setterSink22 = 0;
var syntaxObj22 = {
shorthand22,
plain22: 441,
"string-22": 441 + 2,
1022: 441 + 3,
[computedKey22]: 441 + 4,
...makeSpread(441),
method22(delta) { return this.plain22 + delta; },
get slot22() { return this.plain22 + 8; },
set slot22(value) { setterSink22 = value; },
};
check(syntaxObj22.shorthand22 === 441 + 1, "shorthand syntax 22");
check(syntaxObj22.plain22 === 441, "plain syntax 22");
check(syntaxObj22["string-22"] === 441 + 2, "string key syntax 22");
check(syntaxObj22[1022] === 441 + 3, "numeric key syntax 22");
check(syntaxObj22[computedKey22] === 441 + 4, "computed key syntax 22");
check(syntaxObj22.spreadValue === 441 + 6 && syntaxObj22.shared === 441 + 7, "spread syntax 22");
check(syntaxObj22.method22(9) === 441 + 9, "method syntax 22");
check(syntaxObj22.slot22 === 441 + 8, "getter syntax 22");
syntaxObj22.slot22 = 441 + 10;
check(setterSink22 === 441 + 10, "setter syntax 22");

var shorthand23 = 461 + 1;
var computedKey23 = "computed_23";
var setterSink23 = 0;
var syntaxObj23 = {
shorthand23,
plain23: 461,
"string-23": 461 + 2,
1023: 461 + 3,
[computedKey23]: 461 + 4,
...makeSpread(461),
method23(delta) { return this.plain23 + delta; },
get slot23() { return this.plain23 + 8; },
set slot23(value) { setterSink23 = value; },
};
check(syntaxObj23.shorthand23 === 461 + 1, "shorthand syntax 23");
check(syntaxObj23.plain23 === 461, "plain syntax 23");
check(syntaxObj23["string-23"] === 461 + 2, "string key syntax 23");
check(syntaxObj23[1023] === 461 + 3, "numeric key syntax 23");
check(syntaxObj23[computedKey23] === 461 + 4, "computed key syntax 23");
check(syntaxObj23.spreadValue === 461 + 6 && syntaxObj23.shared === 461 + 7, "spread syntax 23");
check(syntaxObj23.method23(9) === 461 + 9, "method syntax 23");
check(syntaxObj23.slot23 === 461 + 8, "getter syntax 23");
syntaxObj23.slot23 = 461 + 10;
check(setterSink23 === 461 + 10, "setter syntax 23");

var shorthand24 = 481 + 1;
var computedKey24 = "computed_24";
var setterSink24 = 0;
var syntaxObj24 = {
shorthand24,
plain24: 481,
"string-24": 481 + 2,
1024: 481 + 3,
[computedKey24]: 481 + 4,
...makeSpread(481),
method24(delta) { return this.plain24 + delta; },
get slot24() { return this.plain24 + 8; },
set slot24(value) { setterSink24 = value; },
};
check(syntaxObj24.shorthand24 === 481 + 1, "shorthand syntax 24");
check(syntaxObj24.plain24 === 481, "plain syntax 24");
check(syntaxObj24["string-24"] === 481 + 2, "string key syntax 24");
check(syntaxObj24[1024] === 481 + 3, "numeric key syntax 24");
check(syntaxObj24[computedKey24] === 481 + 4, "computed key syntax 24");
check(syntaxObj24.spreadValue === 481 + 6 && syntaxObj24.shared === 481 + 7, "spread syntax 24");
check(syntaxObj24.method24(9) === 481 + 9, "method syntax 24");
check(syntaxObj24.slot24 === 481 + 8, "getter syntax 24");
syntaxObj24.slot24 = 481 + 10;
check(setterSink24 === 481 + 10, "setter syntax 24");

var shorthand25 = 501 + 1;
var computedKey25 = "computed_25";
var setterSink25 = 0;
var syntaxObj25 = {
shorthand25,
plain25: 501,
"string-25": 501 + 2,
1025: 501 + 3,
[computedKey25]: 501 + 4,
...makeSpread(501),
method25(delta) { return this.plain25 + delta; },
get slot25() { return this.plain25 + 8; },
set slot25(value) { setterSink25 = value; },
};
check(syntaxObj25.shorthand25 === 501 + 1, "shorthand syntax 25");
check(syntaxObj25.plain25 === 501, "plain syntax 25");
check(syntaxObj25["string-25"] === 501 + 2, "string key syntax 25");
check(syntaxObj25[1025] === 501 + 3, "numeric key syntax 25");
check(syntaxObj25[computedKey25] === 501 + 4, "computed key syntax 25");
check(syntaxObj25.spreadValue === 501 + 6 && syntaxObj25.shared === 501 + 7, "spread syntax 25");
check(syntaxObj25.method25(9) === 501 + 9, "method syntax 25");
check(syntaxObj25.slot25 === 501 + 8, "getter syntax 25");
syntaxObj25.slot25 = 501 + 10;
check(setterSink25 === 501 + 10, "setter syntax 25");

var shorthand26 = 521 + 1;
var computedKey26 = "computed_26";
var setterSink26 = 0;
var syntaxObj26 = {
shorthand26,
plain26: 521,
"string-26": 521 + 2,
1026: 521 + 3,
[computedKey26]: 521 + 4,
...makeSpread(521),
method26(delta) { return this.plain26 + delta; },
get slot26() { return this.plain26 + 8; },
set slot26(value) { setterSink26 = value; },
};
check(syntaxObj26.shorthand26 === 521 + 1, "shorthand syntax 26");
check(syntaxObj26.plain26 === 521, "plain syntax 26");
check(syntaxObj26["string-26"] === 521 + 2, "string key syntax 26");
check(syntaxObj26[1026] === 521 + 3, "numeric key syntax 26");
check(syntaxObj26[computedKey26] === 521 + 4, "computed key syntax 26");
check(syntaxObj26.spreadValue === 521 + 6 && syntaxObj26.shared === 521 + 7, "spread syntax 26");
check(syntaxObj26.method26(9) === 521 + 9, "method syntax 26");
check(syntaxObj26.slot26 === 521 + 8, "getter syntax 26");
syntaxObj26.slot26 = 521 + 10;
check(setterSink26 === 521 + 10, "setter syntax 26");

var shorthand27 = 541 + 1;
var computedKey27 = "computed_27";
var setterSink27 = 0;
var syntaxObj27 = {
shorthand27,
plain27: 541,
"string-27": 541 + 2,
1027: 541 + 3,
[computedKey27]: 541 + 4,
...makeSpread(541),
method27(delta) { return this.plain27 + delta; },
get slot27() { return this.plain27 + 8; },
set slot27(value) { setterSink27 = value; },
};
check(syntaxObj27.shorthand27 === 541 + 1, "shorthand syntax 27");
check(syntaxObj27.plain27 === 541, "plain syntax 27");
check(syntaxObj27["string-27"] === 541 + 2, "string key syntax 27");
check(syntaxObj27[1027] === 541 + 3, "numeric key syntax 27");
check(syntaxObj27[computedKey27] === 541 + 4, "computed key syntax 27");
check(syntaxObj27.spreadValue === 541 + 6 && syntaxObj27.shared === 541 + 7, "spread syntax 27");
check(syntaxObj27.method27(9) === 541 + 9, "method syntax 27");
check(syntaxObj27.slot27 === 541 + 8, "getter syntax 27");
syntaxObj27.slot27 = 541 + 10;
check(setterSink27 === 541 + 10, "setter syntax 27");

var shorthand28 = 561 + 1;
var computedKey28 = "computed_28";
var setterSink28 = 0;
var syntaxObj28 = {
shorthand28,
plain28: 561,
"string-28": 561 + 2,
1028: 561 + 3,
[computedKey28]: 561 + 4,
...makeSpread(561),
method28(delta) { return this.plain28 + delta; },
get slot28() { return this.plain28 + 8; },
set slot28(value) { setterSink28 = value; },
};
check(syntaxObj28.shorthand28 === 561 + 1, "shorthand syntax 28");
check(syntaxObj28.plain28 === 561, "plain syntax 28");
check(syntaxObj28["string-28"] === 561 + 2, "string key syntax 28");
check(syntaxObj28[1028] === 561 + 3, "numeric key syntax 28");
check(syntaxObj28[computedKey28] === 561 + 4, "computed key syntax 28");
check(syntaxObj28.spreadValue === 561 + 6 && syntaxObj28.shared === 561 + 7, "spread syntax 28");
check(syntaxObj28.method28(9) === 561 + 9, "method syntax 28");
check(syntaxObj28.slot28 === 561 + 8, "getter syntax 28");
syntaxObj28.slot28 = 561 + 10;
check(setterSink28 === 561 + 10, "setter syntax 28");

var shorthand29 = 581 + 1;
var computedKey29 = "computed_29";
var setterSink29 = 0;
var syntaxObj29 = {
shorthand29,
plain29: 581,
"string-29": 581 + 2,
1029: 581 + 3,
[computedKey29]: 581 + 4,
...makeSpread(581),
method29(delta) { return this.plain29 + delta; },
get slot29() { return this.plain29 + 8; },
set slot29(value) { setterSink29 = value; },
};
check(syntaxObj29.shorthand29 === 581 + 1, "shorthand syntax 29");
check(syntaxObj29.plain29 === 581, "plain syntax 29");
check(syntaxObj29["string-29"] === 581 + 2, "string key syntax 29");
check(syntaxObj29[1029] === 581 + 3, "numeric key syntax 29");
check(syntaxObj29[computedKey29] === 581 + 4, "computed key syntax 29");
check(syntaxObj29.spreadValue === 581 + 6 && syntaxObj29.shared === 581 + 7, "spread syntax 29");
check(syntaxObj29.method29(9) === 581 + 9, "method syntax 29");
check(syntaxObj29.slot29 === 581 + 8, "getter syntax 29");
syntaxObj29.slot29 = 581 + 10;
check(setterSink29 === 581 + 10, "setter syntax 29");

var shorthand30 = 601 + 1;
var computedKey30 = "computed_30";
var setterSink30 = 0;
var syntaxObj30 = {
shorthand30,
plain30: 601,
"string-30": 601 + 2,
1030: 601 + 3,
[computedKey30]: 601 + 4,
...makeSpread(601),
method30(delta) { return this.plain30 + delta; },
get slot30() { return this.plain30 + 8; },
set slot30(value) { setterSink30 = value; },
};
check(syntaxObj30.shorthand30 === 601 + 1, "shorthand syntax 30");
check(syntaxObj30.plain30 === 601, "plain syntax 30");
check(syntaxObj30["string-30"] === 601 + 2, "string key syntax 30");
check(syntaxObj30[1030] === 601 + 3, "numeric key syntax 30");
check(syntaxObj30[computedKey30] === 601 + 4, "computed key syntax 30");
check(syntaxObj30.spreadValue === 601 + 6 && syntaxObj30.shared === 601 + 7, "spread syntax 30");
check(syntaxObj30.method30(9) === 601 + 9, "method syntax 30");
check(syntaxObj30.slot30 === 601 + 8, "getter syntax 30");
syntaxObj30.slot30 = 601 + 10;
check(setterSink30 === 601 + 10, "setter syntax 30");

var shorthand31 = 621 + 1;
var computedKey31 = "computed_31";
var setterSink31 = 0;
var syntaxObj31 = {
shorthand31,
plain31: 621,
"string-31": 621 + 2,
1031: 621 + 3,
[computedKey31]: 621 + 4,
...makeSpread(621),
method31(delta) { return this.plain31 + delta; },
get slot31() { return this.plain31 + 8; },
set slot31(value) { setterSink31 = value; },
};
check(syntaxObj31.shorthand31 === 621 + 1, "shorthand syntax 31");
check(syntaxObj31.plain31 === 621, "plain syntax 31");
check(syntaxObj31["string-31"] === 621 + 2, "string key syntax 31");
check(syntaxObj31[1031] === 621 + 3, "numeric key syntax 31");
check(syntaxObj31[computedKey31] === 621 + 4, "computed key syntax 31");
check(syntaxObj31.spreadValue === 621 + 6 && syntaxObj31.shared === 621 + 7, "spread syntax 31");
check(syntaxObj31.method31(9) === 621 + 9, "method syntax 31");
check(syntaxObj31.slot31 === 621 + 8, "getter syntax 31");
syntaxObj31.slot31 = 621 + 10;
check(setterSink31 === 621 + 10, "setter syntax 31");

var shorthand32 = 641 + 1;
var computedKey32 = "computed_32";
var setterSink32 = 0;
var syntaxObj32 = {
shorthand32,
plain32: 641,
"string-32": 641 + 2,
1032: 641 + 3,
[computedKey32]: 641 + 4,
...makeSpread(641),
method32(delta) { return this.plain32 + delta; },
get slot32() { return this.plain32 + 8; },
set slot32(value) { setterSink32 = value; },
};
check(syntaxObj32.shorthand32 === 641 + 1, "shorthand syntax 32");
check(syntaxObj32.plain32 === 641, "plain syntax 32");
check(syntaxObj32["string-32"] === 641 + 2, "string key syntax 32");
check(syntaxObj32[1032] === 641 + 3, "numeric key syntax 32");
check(syntaxObj32[computedKey32] === 641 + 4, "computed key syntax 32");
check(syntaxObj32.spreadValue === 641 + 6 && syntaxObj32.shared === 641 + 7, "spread syntax 32");
check(syntaxObj32.method32(9) === 641 + 9, "method syntax 32");
check(syntaxObj32.slot32 === 641 + 8, "getter syntax 32");
syntaxObj32.slot32 = 641 + 10;
check(setterSink32 === 641 + 10, "setter syntax 32");

var shorthand33 = 661 + 1;
var computedKey33 = "computed_33";
var setterSink33 = 0;
var syntaxObj33 = {
shorthand33,
plain33: 661,
"string-33": 661 + 2,
1033: 661 + 3,
[computedKey33]: 661 + 4,
...makeSpread(661),
method33(delta) { return this.plain33 + delta; },
get slot33() { return this.plain33 + 8; },
set slot33(value) { setterSink33 = value; },
};
check(syntaxObj33.shorthand33 === 661 + 1, "shorthand syntax 33");
check(syntaxObj33.plain33 === 661, "plain syntax 33");
check(syntaxObj33["string-33"] === 661 + 2, "string key syntax 33");
check(syntaxObj33[1033] === 661 + 3, "numeric key syntax 33");
check(syntaxObj33[computedKey33] === 661 + 4, "computed key syntax 33");
check(syntaxObj33.spreadValue === 661 + 6 && syntaxObj33.shared === 661 + 7, "spread syntax 33");
check(syntaxObj33.method33(9) === 661 + 9, "method syntax 33");
check(syntaxObj33.slot33 === 661 + 8, "getter syntax 33");
syntaxObj33.slot33 = 661 + 10;
check(setterSink33 === 661 + 10, "setter syntax 33");

var shorthand34 = 681 + 1;
var computedKey34 = "computed_34";
var setterSink34 = 0;
var syntaxObj34 = {
shorthand34,
plain34: 681,
"string-34": 681 + 2,
1034: 681 + 3,
[computedKey34]: 681 + 4,
...makeSpread(681),
method34(delta) { return this.plain34 + delta; },
get slot34() { return this.plain34 + 8; },
set slot34(value) { setterSink34 = value; },
};
check(syntaxObj34.shorthand34 === 681 + 1, "shorthand syntax 34");
check(syntaxObj34.plain34 === 681, "plain syntax 34");
check(syntaxObj34["string-34"] === 681 + 2, "string key syntax 34");
check(syntaxObj34[1034] === 681 + 3, "numeric key syntax 34");
check(syntaxObj34[computedKey34] === 681 + 4, "computed key syntax 34");
check(syntaxObj34.spreadValue === 681 + 6 && syntaxObj34.shared === 681 + 7, "spread syntax 34");
check(syntaxObj34.method34(9) === 681 + 9, "method syntax 34");
check(syntaxObj34.slot34 === 681 + 8, "getter syntax 34");
syntaxObj34.slot34 = 681 + 10;
check(setterSink34 === 681 + 10, "setter syntax 34");

var shorthand35 = 701 + 1;
var computedKey35 = "computed_35";
var setterSink35 = 0;
var syntaxObj35 = {
shorthand35,
plain35: 701,
"string-35": 701 + 2,
1035: 701 + 3,
[computedKey35]: 701 + 4,
...makeSpread(701),
method35(delta) { return this.plain35 + delta; },
get slot35() { return this.plain35 + 8; },
set slot35(value) { setterSink35 = value; },
};
check(syntaxObj35.shorthand35 === 701 + 1, "shorthand syntax 35");
check(syntaxObj35.plain35 === 701, "plain syntax 35");
check(syntaxObj35["string-35"] === 701 + 2, "string key syntax 35");
check(syntaxObj35[1035] === 701 + 3, "numeric key syntax 35");
check(syntaxObj35[computedKey35] === 701 + 4, "computed key syntax 35");
check(syntaxObj35.spreadValue === 701 + 6 && syntaxObj35.shared === 701 + 7, "spread syntax 35");
check(syntaxObj35.method35(9) === 701 + 9, "method syntax 35");
check(syntaxObj35.slot35 === 701 + 8, "getter syntax 35");
syntaxObj35.slot35 = 701 + 10;
check(setterSink35 === 701 + 10, "setter syntax 35");

var shorthand36 = 721 + 1;
var computedKey36 = "computed_36";
var setterSink36 = 0;
var syntaxObj36 = {
shorthand36,
plain36: 721,
"string-36": 721 + 2,
1036: 721 + 3,
[computedKey36]: 721 + 4,
...makeSpread(721),
method36(delta) { return this.plain36 + delta; },
get slot36() { return this.plain36 + 8; },
set slot36(value) { setterSink36 = value; },
};
check(syntaxObj36.shorthand36 === 721 + 1, "shorthand syntax 36");
check(syntaxObj36.plain36 === 721, "plain syntax 36");
check(syntaxObj36["string-36"] === 721 + 2, "string key syntax 36");
check(syntaxObj36[1036] === 721 + 3, "numeric key syntax 36");
check(syntaxObj36[computedKey36] === 721 + 4, "computed key syntax 36");
check(syntaxObj36.spreadValue === 721 + 6 && syntaxObj36.shared === 721 + 7, "spread syntax 36");
check(syntaxObj36.method36(9) === 721 + 9, "method syntax 36");
check(syntaxObj36.slot36 === 721 + 8, "getter syntax 36");
syntaxObj36.slot36 = 721 + 10;
check(setterSink36 === 721 + 10, "setter syntax 36");

var shorthand37 = 741 + 1;
var computedKey37 = "computed_37";
var setterSink37 = 0;
var syntaxObj37 = {
shorthand37,
plain37: 741,
"string-37": 741 + 2,
1037: 741 + 3,
[computedKey37]: 741 + 4,
...makeSpread(741),
method37(delta) { return this.plain37 + delta; },
get slot37() { return this.plain37 + 8; },
set slot37(value) { setterSink37 = value; },
};
check(syntaxObj37.shorthand37 === 741 + 1, "shorthand syntax 37");
check(syntaxObj37.plain37 === 741, "plain syntax 37");
check(syntaxObj37["string-37"] === 741 + 2, "string key syntax 37");
check(syntaxObj37[1037] === 741 + 3, "numeric key syntax 37");
check(syntaxObj37[computedKey37] === 741 + 4, "computed key syntax 37");
check(syntaxObj37.spreadValue === 741 + 6 && syntaxObj37.shared === 741 + 7, "spread syntax 37");
check(syntaxObj37.method37(9) === 741 + 9, "method syntax 37");
check(syntaxObj37.slot37 === 741 + 8, "getter syntax 37");
syntaxObj37.slot37 = 741 + 10;
check(setterSink37 === 741 + 10, "setter syntax 37");

var shorthand38 = 761 + 1;
var computedKey38 = "computed_38";
var setterSink38 = 0;
var syntaxObj38 = {
shorthand38,
plain38: 761,
"string-38": 761 + 2,
1038: 761 + 3,
[computedKey38]: 761 + 4,
...makeSpread(761),
method38(delta) { return this.plain38 + delta; },
get slot38() { return this.plain38 + 8; },
set slot38(value) { setterSink38 = value; },
};
check(syntaxObj38.shorthand38 === 761 + 1, "shorthand syntax 38");
check(syntaxObj38.plain38 === 761, "plain syntax 38");
check(syntaxObj38["string-38"] === 761 + 2, "string key syntax 38");
check(syntaxObj38[1038] === 761 + 3, "numeric key syntax 38");
check(syntaxObj38[computedKey38] === 761 + 4, "computed key syntax 38");
check(syntaxObj38.spreadValue === 761 + 6 && syntaxObj38.shared === 761 + 7, "spread syntax 38");
check(syntaxObj38.method38(9) === 761 + 9, "method syntax 38");
check(syntaxObj38.slot38 === 761 + 8, "getter syntax 38");
syntaxObj38.slot38 = 761 + 10;
check(setterSink38 === 761 + 10, "setter syntax 38");

var shorthand39 = 781 + 1;
var computedKey39 = "computed_39";
var setterSink39 = 0;
var syntaxObj39 = {
shorthand39,
plain39: 781,
"string-39": 781 + 2,
1039: 781 + 3,
[computedKey39]: 781 + 4,
...makeSpread(781),
method39(delta) { return this.plain39 + delta; },
get slot39() { return this.plain39 + 8; },
set slot39(value) { setterSink39 = value; },
};
check(syntaxObj39.shorthand39 === 781 + 1, "shorthand syntax 39");
check(syntaxObj39.plain39 === 781, "plain syntax 39");
check(syntaxObj39["string-39"] === 781 + 2, "string key syntax 39");
check(syntaxObj39[1039] === 781 + 3, "numeric key syntax 39");
check(syntaxObj39[computedKey39] === 781 + 4, "computed key syntax 39");
check(syntaxObj39.spreadValue === 781 + 6 && syntaxObj39.shared === 781 + 7, "spread syntax 39");
check(syntaxObj39.method39(9) === 781 + 9, "method syntax 39");
check(syntaxObj39.slot39 === 781 + 8, "getter syntax 39");
syntaxObj39.slot39 = 781 + 10;
check(setterSink39 === 781 + 10, "setter syntax 39");

var shorthand40 = 801 + 1;
var computedKey40 = "computed_40";
var setterSink40 = 0;
var syntaxObj40 = {
shorthand40,
plain40: 801,
"string-40": 801 + 2,
1040: 801 + 3,
[computedKey40]: 801 + 4,
...makeSpread(801),
method40(delta) { return this.plain40 + delta; },
get slot40() { return this.plain40 + 8; },
set slot40(value) { setterSink40 = value; },
};
check(syntaxObj40.shorthand40 === 801 + 1, "shorthand syntax 40");
check(syntaxObj40.plain40 === 801, "plain syntax 40");
check(syntaxObj40["string-40"] === 801 + 2, "string key syntax 40");
check(syntaxObj40[1040] === 801 + 3, "numeric key syntax 40");
check(syntaxObj40[computedKey40] === 801 + 4, "computed key syntax 40");
check(syntaxObj40.spreadValue === 801 + 6 && syntaxObj40.shared === 801 + 7, "spread syntax 40");
check(syntaxObj40.method40(9) === 801 + 9, "method syntax 40");
check(syntaxObj40.slot40 === 801 + 8, "getter syntax 40");
syntaxObj40.slot40 = 801 + 10;
check(setterSink40 === 801 + 10, "setter syntax 40");

var shorthand41 = 821 + 1;
var computedKey41 = "computed_41";
var setterSink41 = 0;
var syntaxObj41 = {
shorthand41,
plain41: 821,
"string-41": 821 + 2,
1041: 821 + 3,
[computedKey41]: 821 + 4,
...makeSpread(821),
method41(delta) { return this.plain41 + delta; },
get slot41() { return this.plain41 + 8; },
set slot41(value) { setterSink41 = value; },
};
check(syntaxObj41.shorthand41 === 821 + 1, "shorthand syntax 41");
check(syntaxObj41.plain41 === 821, "plain syntax 41");
check(syntaxObj41["string-41"] === 821 + 2, "string key syntax 41");
check(syntaxObj41[1041] === 821 + 3, "numeric key syntax 41");
check(syntaxObj41[computedKey41] === 821 + 4, "computed key syntax 41");
check(syntaxObj41.spreadValue === 821 + 6 && syntaxObj41.shared === 821 + 7, "spread syntax 41");
check(syntaxObj41.method41(9) === 821 + 9, "method syntax 41");
check(syntaxObj41.slot41 === 821 + 8, "getter syntax 41");
syntaxObj41.slot41 = 821 + 10;
check(setterSink41 === 821 + 10, "setter syntax 41");

var shorthand42 = 841 + 1;
var computedKey42 = "computed_42";
var setterSink42 = 0;
var syntaxObj42 = {
shorthand42,
plain42: 841,
"string-42": 841 + 2,
1042: 841 + 3,
[computedKey42]: 841 + 4,
...makeSpread(841),
method42(delta) { return this.plain42 + delta; },
get slot42() { return this.plain42 + 8; },
set slot42(value) { setterSink42 = value; },
};
check(syntaxObj42.shorthand42 === 841 + 1, "shorthand syntax 42");
check(syntaxObj42.plain42 === 841, "plain syntax 42");
check(syntaxObj42["string-42"] === 841 + 2, "string key syntax 42");
check(syntaxObj42[1042] === 841 + 3, "numeric key syntax 42");
check(syntaxObj42[computedKey42] === 841 + 4, "computed key syntax 42");
check(syntaxObj42.spreadValue === 841 + 6 && syntaxObj42.shared === 841 + 7, "spread syntax 42");
check(syntaxObj42.method42(9) === 841 + 9, "method syntax 42");
check(syntaxObj42.slot42 === 841 + 8, "getter syntax 42");
syntaxObj42.slot42 = 841 + 10;
check(setterSink42 === 841 + 10, "setter syntax 42");

var shorthand43 = 861 + 1;
var computedKey43 = "computed_43";
var setterSink43 = 0;
var syntaxObj43 = {
shorthand43,
plain43: 861,
"string-43": 861 + 2,
1043: 861 + 3,
[computedKey43]: 861 + 4,
...makeSpread(861),
method43(delta) { return this.plain43 + delta; },
get slot43() { return this.plain43 + 8; },
set slot43(value) { setterSink43 = value; },
};
check(syntaxObj43.shorthand43 === 861 + 1, "shorthand syntax 43");
check(syntaxObj43.plain43 === 861, "plain syntax 43");
check(syntaxObj43["string-43"] === 861 + 2, "string key syntax 43");
check(syntaxObj43[1043] === 861 + 3, "numeric key syntax 43");
check(syntaxObj43[computedKey43] === 861 + 4, "computed key syntax 43");
check(syntaxObj43.spreadValue === 861 + 6 && syntaxObj43.shared === 861 + 7, "spread syntax 43");
check(syntaxObj43.method43(9) === 861 + 9, "method syntax 43");
check(syntaxObj43.slot43 === 861 + 8, "getter syntax 43");
syntaxObj43.slot43 = 861 + 10;
check(setterSink43 === 861 + 10, "setter syntax 43");

var shorthand44 = 881 + 1;
var computedKey44 = "computed_44";
var setterSink44 = 0;
var syntaxObj44 = {
shorthand44,
plain44: 881,
"string-44": 881 + 2,
1044: 881 + 3,
[computedKey44]: 881 + 4,
...makeSpread(881),
method44(delta) { return this.plain44 + delta; },
get slot44() { return this.plain44 + 8; },
set slot44(value) { setterSink44 = value; },
};
check(syntaxObj44.shorthand44 === 881 + 1, "shorthand syntax 44");
check(syntaxObj44.plain44 === 881, "plain syntax 44");
check(syntaxObj44["string-44"] === 881 + 2, "string key syntax 44");
check(syntaxObj44[1044] === 881 + 3, "numeric key syntax 44");
check(syntaxObj44[computedKey44] === 881 + 4, "computed key syntax 44");
check(syntaxObj44.spreadValue === 881 + 6 && syntaxObj44.shared === 881 + 7, "spread syntax 44");
check(syntaxObj44.method44(9) === 881 + 9, "method syntax 44");
check(syntaxObj44.slot44 === 881 + 8, "getter syntax 44");
syntaxObj44.slot44 = 881 + 10;
check(setterSink44 === 881 + 10, "setter syntax 44");

check(score > 0, "object literal syntax score");
