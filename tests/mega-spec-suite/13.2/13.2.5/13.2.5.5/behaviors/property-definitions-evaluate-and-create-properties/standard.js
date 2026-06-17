// behavior: property-definitions-evaluate-and-create-properties
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

var orderLog = [];
function mark(label, value) {
orderLog[orderLog.length] = label;
return value;
}
function markObject(label, value) {
orderLog[orderLog.length] = label;
return value;
}

var orderStart0 = orderLog.length;
var shorthandValue0 = mark(40, 40 + 1);
var computedName0 = mark(40 + 2, "computedEval0");
var spreadSource0 = { spreadEval0: 40 + 3, overrideEval0: 40 + 4 };
var protoEval0 = { inheritedEval0: 40 + 5 };
var setterSinkEval0 = 0;
var evalObj0 = {
firstEval0: mark(40 + 6, 40 + 7),
shorthandValue0,
[computedName0]: mark(40 + 8, 40 + 9),
...markObject(40 + 10, spreadSource0),
overrideEval0: mark(40 + 11, 40 + 12),
__proto__: protoEval0,
methodEval0(extra) { return this.firstEval0 + this.overrideEval0 + extra; },
get accessEval0() { return this.firstEval0 + 1; },
set accessEval0(value) { setterSinkEval0 = value; },
};
check(orderLog[orderStart0] === 40, "order shorthand setup 0");
check(orderLog[orderStart0 + 1] === 40 + 2, "order computed name 0");
check(orderLog[orderStart0 + 2] === 40 + 6, "order first value 0");
check(orderLog[orderStart0 + 3] === 40 + 8, "order computed value 0");
check(orderLog[orderStart0 + 4] === 40 + 10, "order spread expression 0");
check(orderLog[orderStart0 + 5] === 40 + 11, "order override value 0");
check(evalObj0.firstEval0 === 40 + 7, "first data property 0");
check(evalObj0.shorthandValue0 === 40 + 1, "shorthand data property 0");
check(evalObj0[computedName0] === 40 + 9, "computed data property 0");
check(evalObj0.spreadEval0 === 40 + 3, "spread data property 0");
check(evalObj0.overrideEval0 === 40 + 12, "later property overrides spread 0");
check(Object.getPrototypeOf(evalObj0) === protoEval0, "literal proto setter 0");
check(evalObj0.inheritedEval0 === 40 + 5, "literal proto inherited 0");
check(evalObj0.methodEval0(2) === (40 + 7) + (40 + 12) + 2, "method definition evaluation 0");
check(evalObj0.accessEval0 === 40 + 8, "getter definition evaluation 0");
evalObj0.accessEval0 = 40 + 14;
check(setterSinkEval0 === 40 + 14, "setter definition evaluation 0");

var orderStart1 = orderLog.length;
var shorthandValue1 = mark(53, 53 + 1);
var computedName1 = mark(53 + 2, "computedEval1");
var spreadSource1 = { spreadEval1: 53 + 3, overrideEval1: 53 + 4 };
var protoEval1 = { inheritedEval1: 53 + 5 };
var setterSinkEval1 = 0;
var evalObj1 = {
firstEval1: mark(53 + 6, 53 + 7),
shorthandValue1,
[computedName1]: mark(53 + 8, 53 + 9),
...markObject(53 + 10, spreadSource1),
overrideEval1: mark(53 + 11, 53 + 12),
__proto__: protoEval1,
methodEval1(extra) { return this.firstEval1 + this.overrideEval1 + extra; },
get accessEval1() { return this.firstEval1 + 1; },
set accessEval1(value) { setterSinkEval1 = value; },
};
check(orderLog[orderStart1] === 53, "order shorthand setup 1");
check(orderLog[orderStart1 + 1] === 53 + 2, "order computed name 1");
check(orderLog[orderStart1 + 2] === 53 + 6, "order first value 1");
check(orderLog[orderStart1 + 3] === 53 + 8, "order computed value 1");
check(orderLog[orderStart1 + 4] === 53 + 10, "order spread expression 1");
check(orderLog[orderStart1 + 5] === 53 + 11, "order override value 1");
check(evalObj1.firstEval1 === 53 + 7, "first data property 1");
check(evalObj1.shorthandValue1 === 53 + 1, "shorthand data property 1");
check(evalObj1[computedName1] === 53 + 9, "computed data property 1");
check(evalObj1.spreadEval1 === 53 + 3, "spread data property 1");
check(evalObj1.overrideEval1 === 53 + 12, "later property overrides spread 1");
check(Object.getPrototypeOf(evalObj1) === protoEval1, "literal proto setter 1");
check(evalObj1.inheritedEval1 === 53 + 5, "literal proto inherited 1");
check(evalObj1.methodEval1(2) === (53 + 7) + (53 + 12) + 2, "method definition evaluation 1");
check(evalObj1.accessEval1 === 53 + 8, "getter definition evaluation 1");
evalObj1.accessEval1 = 53 + 14;
check(setterSinkEval1 === 53 + 14, "setter definition evaluation 1");

var orderStart2 = orderLog.length;
var shorthandValue2 = mark(66, 66 + 1);
var computedName2 = mark(66 + 2, "computedEval2");
var spreadSource2 = { spreadEval2: 66 + 3, overrideEval2: 66 + 4 };
var protoEval2 = { inheritedEval2: 66 + 5 };
var setterSinkEval2 = 0;
var evalObj2 = {
firstEval2: mark(66 + 6, 66 + 7),
shorthandValue2,
[computedName2]: mark(66 + 8, 66 + 9),
...markObject(66 + 10, spreadSource2),
overrideEval2: mark(66 + 11, 66 + 12),
__proto__: protoEval2,
methodEval2(extra) { return this.firstEval2 + this.overrideEval2 + extra; },
get accessEval2() { return this.firstEval2 + 1; },
set accessEval2(value) { setterSinkEval2 = value; },
};
check(orderLog[orderStart2] === 66, "order shorthand setup 2");
check(orderLog[orderStart2 + 1] === 66 + 2, "order computed name 2");
check(orderLog[orderStart2 + 2] === 66 + 6, "order first value 2");
check(orderLog[orderStart2 + 3] === 66 + 8, "order computed value 2");
check(orderLog[orderStart2 + 4] === 66 + 10, "order spread expression 2");
check(orderLog[orderStart2 + 5] === 66 + 11, "order override value 2");
check(evalObj2.firstEval2 === 66 + 7, "first data property 2");
check(evalObj2.shorthandValue2 === 66 + 1, "shorthand data property 2");
check(evalObj2[computedName2] === 66 + 9, "computed data property 2");
check(evalObj2.spreadEval2 === 66 + 3, "spread data property 2");
check(evalObj2.overrideEval2 === 66 + 12, "later property overrides spread 2");
check(Object.getPrototypeOf(evalObj2) === protoEval2, "literal proto setter 2");
check(evalObj2.inheritedEval2 === 66 + 5, "literal proto inherited 2");
check(evalObj2.methodEval2(2) === (66 + 7) + (66 + 12) + 2, "method definition evaluation 2");
check(evalObj2.accessEval2 === 66 + 8, "getter definition evaluation 2");
evalObj2.accessEval2 = 66 + 14;
check(setterSinkEval2 === 66 + 14, "setter definition evaluation 2");

var orderStart3 = orderLog.length;
var shorthandValue3 = mark(79, 79 + 1);
var computedName3 = mark(79 + 2, "computedEval3");
var spreadSource3 = { spreadEval3: 79 + 3, overrideEval3: 79 + 4 };
var protoEval3 = { inheritedEval3: 79 + 5 };
var setterSinkEval3 = 0;
var evalObj3 = {
firstEval3: mark(79 + 6, 79 + 7),
shorthandValue3,
[computedName3]: mark(79 + 8, 79 + 9),
...markObject(79 + 10, spreadSource3),
overrideEval3: mark(79 + 11, 79 + 12),
__proto__: protoEval3,
methodEval3(extra) { return this.firstEval3 + this.overrideEval3 + extra; },
get accessEval3() { return this.firstEval3 + 1; },
set accessEval3(value) { setterSinkEval3 = value; },
};
check(orderLog[orderStart3] === 79, "order shorthand setup 3");
check(orderLog[orderStart3 + 1] === 79 + 2, "order computed name 3");
check(orderLog[orderStart3 + 2] === 79 + 6, "order first value 3");
check(orderLog[orderStart3 + 3] === 79 + 8, "order computed value 3");
check(orderLog[orderStart3 + 4] === 79 + 10, "order spread expression 3");
check(orderLog[orderStart3 + 5] === 79 + 11, "order override value 3");
check(evalObj3.firstEval3 === 79 + 7, "first data property 3");
check(evalObj3.shorthandValue3 === 79 + 1, "shorthand data property 3");
check(evalObj3[computedName3] === 79 + 9, "computed data property 3");
check(evalObj3.spreadEval3 === 79 + 3, "spread data property 3");
check(evalObj3.overrideEval3 === 79 + 12, "later property overrides spread 3");
check(Object.getPrototypeOf(evalObj3) === protoEval3, "literal proto setter 3");
check(evalObj3.inheritedEval3 === 79 + 5, "literal proto inherited 3");
check(evalObj3.methodEval3(2) === (79 + 7) + (79 + 12) + 2, "method definition evaluation 3");
check(evalObj3.accessEval3 === 79 + 8, "getter definition evaluation 3");
evalObj3.accessEval3 = 79 + 14;
check(setterSinkEval3 === 79 + 14, "setter definition evaluation 3");

var orderStart4 = orderLog.length;
var shorthandValue4 = mark(92, 92 + 1);
var computedName4 = mark(92 + 2, "computedEval4");
var spreadSource4 = { spreadEval4: 92 + 3, overrideEval4: 92 + 4 };
var protoEval4 = { inheritedEval4: 92 + 5 };
var setterSinkEval4 = 0;
var evalObj4 = {
firstEval4: mark(92 + 6, 92 + 7),
shorthandValue4,
[computedName4]: mark(92 + 8, 92 + 9),
...markObject(92 + 10, spreadSource4),
overrideEval4: mark(92 + 11, 92 + 12),
__proto__: protoEval4,
methodEval4(extra) { return this.firstEval4 + this.overrideEval4 + extra; },
get accessEval4() { return this.firstEval4 + 1; },
set accessEval4(value) { setterSinkEval4 = value; },
};
check(orderLog[orderStart4] === 92, "order shorthand setup 4");
check(orderLog[orderStart4 + 1] === 92 + 2, "order computed name 4");
check(orderLog[orderStart4 + 2] === 92 + 6, "order first value 4");
check(orderLog[orderStart4 + 3] === 92 + 8, "order computed value 4");
check(orderLog[orderStart4 + 4] === 92 + 10, "order spread expression 4");
check(orderLog[orderStart4 + 5] === 92 + 11, "order override value 4");
check(evalObj4.firstEval4 === 92 + 7, "first data property 4");
check(evalObj4.shorthandValue4 === 92 + 1, "shorthand data property 4");
check(evalObj4[computedName4] === 92 + 9, "computed data property 4");
check(evalObj4.spreadEval4 === 92 + 3, "spread data property 4");
check(evalObj4.overrideEval4 === 92 + 12, "later property overrides spread 4");
check(Object.getPrototypeOf(evalObj4) === protoEval4, "literal proto setter 4");
check(evalObj4.inheritedEval4 === 92 + 5, "literal proto inherited 4");
check(evalObj4.methodEval4(2) === (92 + 7) + (92 + 12) + 2, "method definition evaluation 4");
check(evalObj4.accessEval4 === 92 + 8, "getter definition evaluation 4");
evalObj4.accessEval4 = 92 + 14;
check(setterSinkEval4 === 92 + 14, "setter definition evaluation 4");

var orderStart5 = orderLog.length;
var shorthandValue5 = mark(105, 105 + 1);
var computedName5 = mark(105 + 2, "computedEval5");
var spreadSource5 = { spreadEval5: 105 + 3, overrideEval5: 105 + 4 };
var protoEval5 = { inheritedEval5: 105 + 5 };
var setterSinkEval5 = 0;
var evalObj5 = {
firstEval5: mark(105 + 6, 105 + 7),
shorthandValue5,
[computedName5]: mark(105 + 8, 105 + 9),
...markObject(105 + 10, spreadSource5),
overrideEval5: mark(105 + 11, 105 + 12),
__proto__: protoEval5,
methodEval5(extra) { return this.firstEval5 + this.overrideEval5 + extra; },
get accessEval5() { return this.firstEval5 + 1; },
set accessEval5(value) { setterSinkEval5 = value; },
};
check(orderLog[orderStart5] === 105, "order shorthand setup 5");
check(orderLog[orderStart5 + 1] === 105 + 2, "order computed name 5");
check(orderLog[orderStart5 + 2] === 105 + 6, "order first value 5");
check(orderLog[orderStart5 + 3] === 105 + 8, "order computed value 5");
check(orderLog[orderStart5 + 4] === 105 + 10, "order spread expression 5");
check(orderLog[orderStart5 + 5] === 105 + 11, "order override value 5");
check(evalObj5.firstEval5 === 105 + 7, "first data property 5");
check(evalObj5.shorthandValue5 === 105 + 1, "shorthand data property 5");
check(evalObj5[computedName5] === 105 + 9, "computed data property 5");
check(evalObj5.spreadEval5 === 105 + 3, "spread data property 5");
check(evalObj5.overrideEval5 === 105 + 12, "later property overrides spread 5");
check(Object.getPrototypeOf(evalObj5) === protoEval5, "literal proto setter 5");
check(evalObj5.inheritedEval5 === 105 + 5, "literal proto inherited 5");
check(evalObj5.methodEval5(2) === (105 + 7) + (105 + 12) + 2, "method definition evaluation 5");
check(evalObj5.accessEval5 === 105 + 8, "getter definition evaluation 5");
evalObj5.accessEval5 = 105 + 14;
check(setterSinkEval5 === 105 + 14, "setter definition evaluation 5");

var orderStart6 = orderLog.length;
var shorthandValue6 = mark(118, 118 + 1);
var computedName6 = mark(118 + 2, "computedEval6");
var spreadSource6 = { spreadEval6: 118 + 3, overrideEval6: 118 + 4 };
var protoEval6 = { inheritedEval6: 118 + 5 };
var setterSinkEval6 = 0;
var evalObj6 = {
firstEval6: mark(118 + 6, 118 + 7),
shorthandValue6,
[computedName6]: mark(118 + 8, 118 + 9),
...markObject(118 + 10, spreadSource6),
overrideEval6: mark(118 + 11, 118 + 12),
__proto__: protoEval6,
methodEval6(extra) { return this.firstEval6 + this.overrideEval6 + extra; },
get accessEval6() { return this.firstEval6 + 1; },
set accessEval6(value) { setterSinkEval6 = value; },
};
check(orderLog[orderStart6] === 118, "order shorthand setup 6");
check(orderLog[orderStart6 + 1] === 118 + 2, "order computed name 6");
check(orderLog[orderStart6 + 2] === 118 + 6, "order first value 6");
check(orderLog[orderStart6 + 3] === 118 + 8, "order computed value 6");
check(orderLog[orderStart6 + 4] === 118 + 10, "order spread expression 6");
check(orderLog[orderStart6 + 5] === 118 + 11, "order override value 6");
check(evalObj6.firstEval6 === 118 + 7, "first data property 6");
check(evalObj6.shorthandValue6 === 118 + 1, "shorthand data property 6");
check(evalObj6[computedName6] === 118 + 9, "computed data property 6");
check(evalObj6.spreadEval6 === 118 + 3, "spread data property 6");
check(evalObj6.overrideEval6 === 118 + 12, "later property overrides spread 6");
check(Object.getPrototypeOf(evalObj6) === protoEval6, "literal proto setter 6");
check(evalObj6.inheritedEval6 === 118 + 5, "literal proto inherited 6");
check(evalObj6.methodEval6(2) === (118 + 7) + (118 + 12) + 2, "method definition evaluation 6");
check(evalObj6.accessEval6 === 118 + 8, "getter definition evaluation 6");
evalObj6.accessEval6 = 118 + 14;
check(setterSinkEval6 === 118 + 14, "setter definition evaluation 6");

var orderStart7 = orderLog.length;
var shorthandValue7 = mark(131, 131 + 1);
var computedName7 = mark(131 + 2, "computedEval7");
var spreadSource7 = { spreadEval7: 131 + 3, overrideEval7: 131 + 4 };
var protoEval7 = { inheritedEval7: 131 + 5 };
var setterSinkEval7 = 0;
var evalObj7 = {
firstEval7: mark(131 + 6, 131 + 7),
shorthandValue7,
[computedName7]: mark(131 + 8, 131 + 9),
...markObject(131 + 10, spreadSource7),
overrideEval7: mark(131 + 11, 131 + 12),
__proto__: protoEval7,
methodEval7(extra) { return this.firstEval7 + this.overrideEval7 + extra; },
get accessEval7() { return this.firstEval7 + 1; },
set accessEval7(value) { setterSinkEval7 = value; },
};
check(orderLog[orderStart7] === 131, "order shorthand setup 7");
check(orderLog[orderStart7 + 1] === 131 + 2, "order computed name 7");
check(orderLog[orderStart7 + 2] === 131 + 6, "order first value 7");
check(orderLog[orderStart7 + 3] === 131 + 8, "order computed value 7");
check(orderLog[orderStart7 + 4] === 131 + 10, "order spread expression 7");
check(orderLog[orderStart7 + 5] === 131 + 11, "order override value 7");
check(evalObj7.firstEval7 === 131 + 7, "first data property 7");
check(evalObj7.shorthandValue7 === 131 + 1, "shorthand data property 7");
check(evalObj7[computedName7] === 131 + 9, "computed data property 7");
check(evalObj7.spreadEval7 === 131 + 3, "spread data property 7");
check(evalObj7.overrideEval7 === 131 + 12, "later property overrides spread 7");
check(Object.getPrototypeOf(evalObj7) === protoEval7, "literal proto setter 7");
check(evalObj7.inheritedEval7 === 131 + 5, "literal proto inherited 7");
check(evalObj7.methodEval7(2) === (131 + 7) + (131 + 12) + 2, "method definition evaluation 7");
check(evalObj7.accessEval7 === 131 + 8, "getter definition evaluation 7");
evalObj7.accessEval7 = 131 + 14;
check(setterSinkEval7 === 131 + 14, "setter definition evaluation 7");

var orderStart8 = orderLog.length;
var shorthandValue8 = mark(144, 144 + 1);
var computedName8 = mark(144 + 2, "computedEval8");
var spreadSource8 = { spreadEval8: 144 + 3, overrideEval8: 144 + 4 };
var protoEval8 = { inheritedEval8: 144 + 5 };
var setterSinkEval8 = 0;
var evalObj8 = {
firstEval8: mark(144 + 6, 144 + 7),
shorthandValue8,
[computedName8]: mark(144 + 8, 144 + 9),
...markObject(144 + 10, spreadSource8),
overrideEval8: mark(144 + 11, 144 + 12),
__proto__: protoEval8,
methodEval8(extra) { return this.firstEval8 + this.overrideEval8 + extra; },
get accessEval8() { return this.firstEval8 + 1; },
set accessEval8(value) { setterSinkEval8 = value; },
};
check(orderLog[orderStart8] === 144, "order shorthand setup 8");
check(orderLog[orderStart8 + 1] === 144 + 2, "order computed name 8");
check(orderLog[orderStart8 + 2] === 144 + 6, "order first value 8");
check(orderLog[orderStart8 + 3] === 144 + 8, "order computed value 8");
check(orderLog[orderStart8 + 4] === 144 + 10, "order spread expression 8");
check(orderLog[orderStart8 + 5] === 144 + 11, "order override value 8");
check(evalObj8.firstEval8 === 144 + 7, "first data property 8");
check(evalObj8.shorthandValue8 === 144 + 1, "shorthand data property 8");
check(evalObj8[computedName8] === 144 + 9, "computed data property 8");
check(evalObj8.spreadEval8 === 144 + 3, "spread data property 8");
check(evalObj8.overrideEval8 === 144 + 12, "later property overrides spread 8");
check(Object.getPrototypeOf(evalObj8) === protoEval8, "literal proto setter 8");
check(evalObj8.inheritedEval8 === 144 + 5, "literal proto inherited 8");
check(evalObj8.methodEval8(2) === (144 + 7) + (144 + 12) + 2, "method definition evaluation 8");
check(evalObj8.accessEval8 === 144 + 8, "getter definition evaluation 8");
evalObj8.accessEval8 = 144 + 14;
check(setterSinkEval8 === 144 + 14, "setter definition evaluation 8");

var orderStart9 = orderLog.length;
var shorthandValue9 = mark(157, 157 + 1);
var computedName9 = mark(157 + 2, "computedEval9");
var spreadSource9 = { spreadEval9: 157 + 3, overrideEval9: 157 + 4 };
var protoEval9 = { inheritedEval9: 157 + 5 };
var setterSinkEval9 = 0;
var evalObj9 = {
firstEval9: mark(157 + 6, 157 + 7),
shorthandValue9,
[computedName9]: mark(157 + 8, 157 + 9),
...markObject(157 + 10, spreadSource9),
overrideEval9: mark(157 + 11, 157 + 12),
__proto__: protoEval9,
methodEval9(extra) { return this.firstEval9 + this.overrideEval9 + extra; },
get accessEval9() { return this.firstEval9 + 1; },
set accessEval9(value) { setterSinkEval9 = value; },
};
check(orderLog[orderStart9] === 157, "order shorthand setup 9");
check(orderLog[orderStart9 + 1] === 157 + 2, "order computed name 9");
check(orderLog[orderStart9 + 2] === 157 + 6, "order first value 9");
check(orderLog[orderStart9 + 3] === 157 + 8, "order computed value 9");
check(orderLog[orderStart9 + 4] === 157 + 10, "order spread expression 9");
check(orderLog[orderStart9 + 5] === 157 + 11, "order override value 9");
check(evalObj9.firstEval9 === 157 + 7, "first data property 9");
check(evalObj9.shorthandValue9 === 157 + 1, "shorthand data property 9");
check(evalObj9[computedName9] === 157 + 9, "computed data property 9");
check(evalObj9.spreadEval9 === 157 + 3, "spread data property 9");
check(evalObj9.overrideEval9 === 157 + 12, "later property overrides spread 9");
check(Object.getPrototypeOf(evalObj9) === protoEval9, "literal proto setter 9");
check(evalObj9.inheritedEval9 === 157 + 5, "literal proto inherited 9");
check(evalObj9.methodEval9(2) === (157 + 7) + (157 + 12) + 2, "method definition evaluation 9");
check(evalObj9.accessEval9 === 157 + 8, "getter definition evaluation 9");
evalObj9.accessEval9 = 157 + 14;
check(setterSinkEval9 === 157 + 14, "setter definition evaluation 9");

check(score > 0, "property definition evaluation score");
