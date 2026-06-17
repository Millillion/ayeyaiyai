// behavior: property-definitions-evaluate-and-create-properties
// expected: pass
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

var orderStart10 = orderLog.length;
var shorthandValue10 = mark(170, 170 + 1);
var computedName10 = mark(170 + 2, "computedEval10");
var spreadSource10 = { spreadEval10: 170 + 3, overrideEval10: 170 + 4 };
var protoEval10 = { inheritedEval10: 170 + 5 };
var setterSinkEval10 = 0;
var evalObj10 = {
firstEval10: mark(170 + 6, 170 + 7),
shorthandValue10,
[computedName10]: mark(170 + 8, 170 + 9),
...markObject(170 + 10, spreadSource10),
overrideEval10: mark(170 + 11, 170 + 12),
__proto__: protoEval10,
methodEval10(extra) { return this.firstEval10 + this.overrideEval10 + extra; },
get accessEval10() { return this.firstEval10 + 1; },
set accessEval10(value) { setterSinkEval10 = value; },
};
check(orderLog[orderStart10] === 170, "order shorthand setup 10");
check(orderLog[orderStart10 + 1] === 170 + 2, "order computed name 10");
check(orderLog[orderStart10 + 2] === 170 + 6, "order first value 10");
check(orderLog[orderStart10 + 3] === 170 + 8, "order computed value 10");
check(orderLog[orderStart10 + 4] === 170 + 10, "order spread expression 10");
check(orderLog[orderStart10 + 5] === 170 + 11, "order override value 10");
check(evalObj10.firstEval10 === 170 + 7, "first data property 10");
check(evalObj10.shorthandValue10 === 170 + 1, "shorthand data property 10");
check(evalObj10[computedName10] === 170 + 9, "computed data property 10");
check(evalObj10.spreadEval10 === 170 + 3, "spread data property 10");
check(evalObj10.overrideEval10 === 170 + 12, "later property overrides spread 10");
check(Object.getPrototypeOf(evalObj10) === protoEval10, "literal proto setter 10");
check(evalObj10.inheritedEval10 === 170 + 5, "literal proto inherited 10");
check(evalObj10.methodEval10(2) === (170 + 7) + (170 + 12) + 2, "method definition evaluation 10");
check(evalObj10.accessEval10 === 170 + 8, "getter definition evaluation 10");
evalObj10.accessEval10 = 170 + 14;
check(setterSinkEval10 === 170 + 14, "setter definition evaluation 10");

var orderStart11 = orderLog.length;
var shorthandValue11 = mark(183, 183 + 1);
var computedName11 = mark(183 + 2, "computedEval11");
var spreadSource11 = { spreadEval11: 183 + 3, overrideEval11: 183 + 4 };
var protoEval11 = { inheritedEval11: 183 + 5 };
var setterSinkEval11 = 0;
var evalObj11 = {
firstEval11: mark(183 + 6, 183 + 7),
shorthandValue11,
[computedName11]: mark(183 + 8, 183 + 9),
...markObject(183 + 10, spreadSource11),
overrideEval11: mark(183 + 11, 183 + 12),
__proto__: protoEval11,
methodEval11(extra) { return this.firstEval11 + this.overrideEval11 + extra; },
get accessEval11() { return this.firstEval11 + 1; },
set accessEval11(value) { setterSinkEval11 = value; },
};
check(orderLog[orderStart11] === 183, "order shorthand setup 11");
check(orderLog[orderStart11 + 1] === 183 + 2, "order computed name 11");
check(orderLog[orderStart11 + 2] === 183 + 6, "order first value 11");
check(orderLog[orderStart11 + 3] === 183 + 8, "order computed value 11");
check(orderLog[orderStart11 + 4] === 183 + 10, "order spread expression 11");
check(orderLog[orderStart11 + 5] === 183 + 11, "order override value 11");
check(evalObj11.firstEval11 === 183 + 7, "first data property 11");
check(evalObj11.shorthandValue11 === 183 + 1, "shorthand data property 11");
check(evalObj11[computedName11] === 183 + 9, "computed data property 11");
check(evalObj11.spreadEval11 === 183 + 3, "spread data property 11");
check(evalObj11.overrideEval11 === 183 + 12, "later property overrides spread 11");
check(Object.getPrototypeOf(evalObj11) === protoEval11, "literal proto setter 11");
check(evalObj11.inheritedEval11 === 183 + 5, "literal proto inherited 11");
check(evalObj11.methodEval11(2) === (183 + 7) + (183 + 12) + 2, "method definition evaluation 11");
check(evalObj11.accessEval11 === 183 + 8, "getter definition evaluation 11");
evalObj11.accessEval11 = 183 + 14;
check(setterSinkEval11 === 183 + 14, "setter definition evaluation 11");

var orderStart12 = orderLog.length;
var shorthandValue12 = mark(196, 196 + 1);
var computedName12 = mark(196 + 2, "computedEval12");
var spreadSource12 = { spreadEval12: 196 + 3, overrideEval12: 196 + 4 };
var protoEval12 = { inheritedEval12: 196 + 5 };
var setterSinkEval12 = 0;
var evalObj12 = {
firstEval12: mark(196 + 6, 196 + 7),
shorthandValue12,
[computedName12]: mark(196 + 8, 196 + 9),
...markObject(196 + 10, spreadSource12),
overrideEval12: mark(196 + 11, 196 + 12),
__proto__: protoEval12,
methodEval12(extra) { return this.firstEval12 + this.overrideEval12 + extra; },
get accessEval12() { return this.firstEval12 + 1; },
set accessEval12(value) { setterSinkEval12 = value; },
};
check(orderLog[orderStart12] === 196, "order shorthand setup 12");
check(orderLog[orderStart12 + 1] === 196 + 2, "order computed name 12");
check(orderLog[orderStart12 + 2] === 196 + 6, "order first value 12");
check(orderLog[orderStart12 + 3] === 196 + 8, "order computed value 12");
check(orderLog[orderStart12 + 4] === 196 + 10, "order spread expression 12");
check(orderLog[orderStart12 + 5] === 196 + 11, "order override value 12");
check(evalObj12.firstEval12 === 196 + 7, "first data property 12");
check(evalObj12.shorthandValue12 === 196 + 1, "shorthand data property 12");
check(evalObj12[computedName12] === 196 + 9, "computed data property 12");
check(evalObj12.spreadEval12 === 196 + 3, "spread data property 12");
check(evalObj12.overrideEval12 === 196 + 12, "later property overrides spread 12");
check(Object.getPrototypeOf(evalObj12) === protoEval12, "literal proto setter 12");
check(evalObj12.inheritedEval12 === 196 + 5, "literal proto inherited 12");
check(evalObj12.methodEval12(2) === (196 + 7) + (196 + 12) + 2, "method definition evaluation 12");
check(evalObj12.accessEval12 === 196 + 8, "getter definition evaluation 12");
evalObj12.accessEval12 = 196 + 14;
check(setterSinkEval12 === 196 + 14, "setter definition evaluation 12");

var orderStart13 = orderLog.length;
var shorthandValue13 = mark(209, 209 + 1);
var computedName13 = mark(209 + 2, "computedEval13");
var spreadSource13 = { spreadEval13: 209 + 3, overrideEval13: 209 + 4 };
var protoEval13 = { inheritedEval13: 209 + 5 };
var setterSinkEval13 = 0;
var evalObj13 = {
firstEval13: mark(209 + 6, 209 + 7),
shorthandValue13,
[computedName13]: mark(209 + 8, 209 + 9),
...markObject(209 + 10, spreadSource13),
overrideEval13: mark(209 + 11, 209 + 12),
__proto__: protoEval13,
methodEval13(extra) { return this.firstEval13 + this.overrideEval13 + extra; },
get accessEval13() { return this.firstEval13 + 1; },
set accessEval13(value) { setterSinkEval13 = value; },
};
check(orderLog[orderStart13] === 209, "order shorthand setup 13");
check(orderLog[orderStart13 + 1] === 209 + 2, "order computed name 13");
check(orderLog[orderStart13 + 2] === 209 + 6, "order first value 13");
check(orderLog[orderStart13 + 3] === 209 + 8, "order computed value 13");
check(orderLog[orderStart13 + 4] === 209 + 10, "order spread expression 13");
check(orderLog[orderStart13 + 5] === 209 + 11, "order override value 13");
check(evalObj13.firstEval13 === 209 + 7, "first data property 13");
check(evalObj13.shorthandValue13 === 209 + 1, "shorthand data property 13");
check(evalObj13[computedName13] === 209 + 9, "computed data property 13");
check(evalObj13.spreadEval13 === 209 + 3, "spread data property 13");
check(evalObj13.overrideEval13 === 209 + 12, "later property overrides spread 13");
check(Object.getPrototypeOf(evalObj13) === protoEval13, "literal proto setter 13");
check(evalObj13.inheritedEval13 === 209 + 5, "literal proto inherited 13");
check(evalObj13.methodEval13(2) === (209 + 7) + (209 + 12) + 2, "method definition evaluation 13");
check(evalObj13.accessEval13 === 209 + 8, "getter definition evaluation 13");
evalObj13.accessEval13 = 209 + 14;
check(setterSinkEval13 === 209 + 14, "setter definition evaluation 13");

var orderStart14 = orderLog.length;
var shorthandValue14 = mark(222, 222 + 1);
var computedName14 = mark(222 + 2, "computedEval14");
var spreadSource14 = { spreadEval14: 222 + 3, overrideEval14: 222 + 4 };
var protoEval14 = { inheritedEval14: 222 + 5 };
var setterSinkEval14 = 0;
var evalObj14 = {
firstEval14: mark(222 + 6, 222 + 7),
shorthandValue14,
[computedName14]: mark(222 + 8, 222 + 9),
...markObject(222 + 10, spreadSource14),
overrideEval14: mark(222 + 11, 222 + 12),
__proto__: protoEval14,
methodEval14(extra) { return this.firstEval14 + this.overrideEval14 + extra; },
get accessEval14() { return this.firstEval14 + 1; },
set accessEval14(value) { setterSinkEval14 = value; },
};
check(orderLog[orderStart14] === 222, "order shorthand setup 14");
check(orderLog[orderStart14 + 1] === 222 + 2, "order computed name 14");
check(orderLog[orderStart14 + 2] === 222 + 6, "order first value 14");
check(orderLog[orderStart14 + 3] === 222 + 8, "order computed value 14");
check(orderLog[orderStart14 + 4] === 222 + 10, "order spread expression 14");
check(orderLog[orderStart14 + 5] === 222 + 11, "order override value 14");
check(evalObj14.firstEval14 === 222 + 7, "first data property 14");
check(evalObj14.shorthandValue14 === 222 + 1, "shorthand data property 14");
check(evalObj14[computedName14] === 222 + 9, "computed data property 14");
check(evalObj14.spreadEval14 === 222 + 3, "spread data property 14");
check(evalObj14.overrideEval14 === 222 + 12, "later property overrides spread 14");
check(Object.getPrototypeOf(evalObj14) === protoEval14, "literal proto setter 14");
check(evalObj14.inheritedEval14 === 222 + 5, "literal proto inherited 14");
check(evalObj14.methodEval14(2) === (222 + 7) + (222 + 12) + 2, "method definition evaluation 14");
check(evalObj14.accessEval14 === 222 + 8, "getter definition evaluation 14");
evalObj14.accessEval14 = 222 + 14;
check(setterSinkEval14 === 222 + 14, "setter definition evaluation 14");

var orderStart15 = orderLog.length;
var shorthandValue15 = mark(235, 235 + 1);
var computedName15 = mark(235 + 2, "computedEval15");
var spreadSource15 = { spreadEval15: 235 + 3, overrideEval15: 235 + 4 };
var protoEval15 = { inheritedEval15: 235 + 5 };
var setterSinkEval15 = 0;
var evalObj15 = {
firstEval15: mark(235 + 6, 235 + 7),
shorthandValue15,
[computedName15]: mark(235 + 8, 235 + 9),
...markObject(235 + 10, spreadSource15),
overrideEval15: mark(235 + 11, 235 + 12),
__proto__: protoEval15,
methodEval15(extra) { return this.firstEval15 + this.overrideEval15 + extra; },
get accessEval15() { return this.firstEval15 + 1; },
set accessEval15(value) { setterSinkEval15 = value; },
};
check(orderLog[orderStart15] === 235, "order shorthand setup 15");
check(orderLog[orderStart15 + 1] === 235 + 2, "order computed name 15");
check(orderLog[orderStart15 + 2] === 235 + 6, "order first value 15");
check(orderLog[orderStart15 + 3] === 235 + 8, "order computed value 15");
check(orderLog[orderStart15 + 4] === 235 + 10, "order spread expression 15");
check(orderLog[orderStart15 + 5] === 235 + 11, "order override value 15");
check(evalObj15.firstEval15 === 235 + 7, "first data property 15");
check(evalObj15.shorthandValue15 === 235 + 1, "shorthand data property 15");
check(evalObj15[computedName15] === 235 + 9, "computed data property 15");
check(evalObj15.spreadEval15 === 235 + 3, "spread data property 15");
check(evalObj15.overrideEval15 === 235 + 12, "later property overrides spread 15");
check(Object.getPrototypeOf(evalObj15) === protoEval15, "literal proto setter 15");
check(evalObj15.inheritedEval15 === 235 + 5, "literal proto inherited 15");
check(evalObj15.methodEval15(2) === (235 + 7) + (235 + 12) + 2, "method definition evaluation 15");
check(evalObj15.accessEval15 === 235 + 8, "getter definition evaluation 15");
evalObj15.accessEval15 = 235 + 14;
check(setterSinkEval15 === 235 + 14, "setter definition evaluation 15");

var orderStart16 = orderLog.length;
var shorthandValue16 = mark(248, 248 + 1);
var computedName16 = mark(248 + 2, "computedEval16");
var spreadSource16 = { spreadEval16: 248 + 3, overrideEval16: 248 + 4 };
var protoEval16 = { inheritedEval16: 248 + 5 };
var setterSinkEval16 = 0;
var evalObj16 = {
firstEval16: mark(248 + 6, 248 + 7),
shorthandValue16,
[computedName16]: mark(248 + 8, 248 + 9),
...markObject(248 + 10, spreadSource16),
overrideEval16: mark(248 + 11, 248 + 12),
__proto__: protoEval16,
methodEval16(extra) { return this.firstEval16 + this.overrideEval16 + extra; },
get accessEval16() { return this.firstEval16 + 1; },
set accessEval16(value) { setterSinkEval16 = value; },
};
check(orderLog[orderStart16] === 248, "order shorthand setup 16");
check(orderLog[orderStart16 + 1] === 248 + 2, "order computed name 16");
check(orderLog[orderStart16 + 2] === 248 + 6, "order first value 16");
check(orderLog[orderStart16 + 3] === 248 + 8, "order computed value 16");
check(orderLog[orderStart16 + 4] === 248 + 10, "order spread expression 16");
check(orderLog[orderStart16 + 5] === 248 + 11, "order override value 16");
check(evalObj16.firstEval16 === 248 + 7, "first data property 16");
check(evalObj16.shorthandValue16 === 248 + 1, "shorthand data property 16");
check(evalObj16[computedName16] === 248 + 9, "computed data property 16");
check(evalObj16.spreadEval16 === 248 + 3, "spread data property 16");
check(evalObj16.overrideEval16 === 248 + 12, "later property overrides spread 16");
check(Object.getPrototypeOf(evalObj16) === protoEval16, "literal proto setter 16");
check(evalObj16.inheritedEval16 === 248 + 5, "literal proto inherited 16");
check(evalObj16.methodEval16(2) === (248 + 7) + (248 + 12) + 2, "method definition evaluation 16");
check(evalObj16.accessEval16 === 248 + 8, "getter definition evaluation 16");
evalObj16.accessEval16 = 248 + 14;
check(setterSinkEval16 === 248 + 14, "setter definition evaluation 16");

var orderStart17 = orderLog.length;
var shorthandValue17 = mark(261, 261 + 1);
var computedName17 = mark(261 + 2, "computedEval17");
var spreadSource17 = { spreadEval17: 261 + 3, overrideEval17: 261 + 4 };
var protoEval17 = { inheritedEval17: 261 + 5 };
var setterSinkEval17 = 0;
var evalObj17 = {
firstEval17: mark(261 + 6, 261 + 7),
shorthandValue17,
[computedName17]: mark(261 + 8, 261 + 9),
...markObject(261 + 10, spreadSource17),
overrideEval17: mark(261 + 11, 261 + 12),
__proto__: protoEval17,
methodEval17(extra) { return this.firstEval17 + this.overrideEval17 + extra; },
get accessEval17() { return this.firstEval17 + 1; },
set accessEval17(value) { setterSinkEval17 = value; },
};
check(orderLog[orderStart17] === 261, "order shorthand setup 17");
check(orderLog[orderStart17 + 1] === 261 + 2, "order computed name 17");
check(orderLog[orderStart17 + 2] === 261 + 6, "order first value 17");
check(orderLog[orderStart17 + 3] === 261 + 8, "order computed value 17");
check(orderLog[orderStart17 + 4] === 261 + 10, "order spread expression 17");
check(orderLog[orderStart17 + 5] === 261 + 11, "order override value 17");
check(evalObj17.firstEval17 === 261 + 7, "first data property 17");
check(evalObj17.shorthandValue17 === 261 + 1, "shorthand data property 17");
check(evalObj17[computedName17] === 261 + 9, "computed data property 17");
check(evalObj17.spreadEval17 === 261 + 3, "spread data property 17");
check(evalObj17.overrideEval17 === 261 + 12, "later property overrides spread 17");
check(Object.getPrototypeOf(evalObj17) === protoEval17, "literal proto setter 17");
check(evalObj17.inheritedEval17 === 261 + 5, "literal proto inherited 17");
check(evalObj17.methodEval17(2) === (261 + 7) + (261 + 12) + 2, "method definition evaluation 17");
check(evalObj17.accessEval17 === 261 + 8, "getter definition evaluation 17");
evalObj17.accessEval17 = 261 + 14;
check(setterSinkEval17 === 261 + 14, "setter definition evaluation 17");

var orderStart18 = orderLog.length;
var shorthandValue18 = mark(274, 274 + 1);
var computedName18 = mark(274 + 2, "computedEval18");
var spreadSource18 = { spreadEval18: 274 + 3, overrideEval18: 274 + 4 };
var protoEval18 = { inheritedEval18: 274 + 5 };
var setterSinkEval18 = 0;
var evalObj18 = {
firstEval18: mark(274 + 6, 274 + 7),
shorthandValue18,
[computedName18]: mark(274 + 8, 274 + 9),
...markObject(274 + 10, spreadSource18),
overrideEval18: mark(274 + 11, 274 + 12),
__proto__: protoEval18,
methodEval18(extra) { return this.firstEval18 + this.overrideEval18 + extra; },
get accessEval18() { return this.firstEval18 + 1; },
set accessEval18(value) { setterSinkEval18 = value; },
};
check(orderLog[orderStart18] === 274, "order shorthand setup 18");
check(orderLog[orderStart18 + 1] === 274 + 2, "order computed name 18");
check(orderLog[orderStart18 + 2] === 274 + 6, "order first value 18");
check(orderLog[orderStart18 + 3] === 274 + 8, "order computed value 18");
check(orderLog[orderStart18 + 4] === 274 + 10, "order spread expression 18");
check(orderLog[orderStart18 + 5] === 274 + 11, "order override value 18");
check(evalObj18.firstEval18 === 274 + 7, "first data property 18");
check(evalObj18.shorthandValue18 === 274 + 1, "shorthand data property 18");
check(evalObj18[computedName18] === 274 + 9, "computed data property 18");
check(evalObj18.spreadEval18 === 274 + 3, "spread data property 18");
check(evalObj18.overrideEval18 === 274 + 12, "later property overrides spread 18");
check(Object.getPrototypeOf(evalObj18) === protoEval18, "literal proto setter 18");
check(evalObj18.inheritedEval18 === 274 + 5, "literal proto inherited 18");
check(evalObj18.methodEval18(2) === (274 + 7) + (274 + 12) + 2, "method definition evaluation 18");
check(evalObj18.accessEval18 === 274 + 8, "getter definition evaluation 18");
evalObj18.accessEval18 = 274 + 14;
check(setterSinkEval18 === 274 + 14, "setter definition evaluation 18");

var orderStart19 = orderLog.length;
var shorthandValue19 = mark(287, 287 + 1);
var computedName19 = mark(287 + 2, "computedEval19");
var spreadSource19 = { spreadEval19: 287 + 3, overrideEval19: 287 + 4 };
var protoEval19 = { inheritedEval19: 287 + 5 };
var setterSinkEval19 = 0;
var evalObj19 = {
firstEval19: mark(287 + 6, 287 + 7),
shorthandValue19,
[computedName19]: mark(287 + 8, 287 + 9),
...markObject(287 + 10, spreadSource19),
overrideEval19: mark(287 + 11, 287 + 12),
__proto__: protoEval19,
methodEval19(extra) { return this.firstEval19 + this.overrideEval19 + extra; },
get accessEval19() { return this.firstEval19 + 1; },
set accessEval19(value) { setterSinkEval19 = value; },
};
check(orderLog[orderStart19] === 287, "order shorthand setup 19");
check(orderLog[orderStart19 + 1] === 287 + 2, "order computed name 19");
check(orderLog[orderStart19 + 2] === 287 + 6, "order first value 19");
check(orderLog[orderStart19 + 3] === 287 + 8, "order computed value 19");
check(orderLog[orderStart19 + 4] === 287 + 10, "order spread expression 19");
check(orderLog[orderStart19 + 5] === 287 + 11, "order override value 19");
check(evalObj19.firstEval19 === 287 + 7, "first data property 19");
check(evalObj19.shorthandValue19 === 287 + 1, "shorthand data property 19");
check(evalObj19[computedName19] === 287 + 9, "computed data property 19");
check(evalObj19.spreadEval19 === 287 + 3, "spread data property 19");
check(evalObj19.overrideEval19 === 287 + 12, "later property overrides spread 19");
check(Object.getPrototypeOf(evalObj19) === protoEval19, "literal proto setter 19");
check(evalObj19.inheritedEval19 === 287 + 5, "literal proto inherited 19");
check(evalObj19.methodEval19(2) === (287 + 7) + (287 + 12) + 2, "method definition evaluation 19");
check(evalObj19.accessEval19 === 287 + 8, "getter definition evaluation 19");
evalObj19.accessEval19 = 287 + 14;
check(setterSinkEval19 === 287 + 14, "setter definition evaluation 19");

var orderStart20 = orderLog.length;
var shorthandValue20 = mark(300, 300 + 1);
var computedName20 = mark(300 + 2, "computedEval20");
var spreadSource20 = { spreadEval20: 300 + 3, overrideEval20: 300 + 4 };
var protoEval20 = { inheritedEval20: 300 + 5 };
var setterSinkEval20 = 0;
var evalObj20 = {
firstEval20: mark(300 + 6, 300 + 7),
shorthandValue20,
[computedName20]: mark(300 + 8, 300 + 9),
...markObject(300 + 10, spreadSource20),
overrideEval20: mark(300 + 11, 300 + 12),
__proto__: protoEval20,
methodEval20(extra) { return this.firstEval20 + this.overrideEval20 + extra; },
get accessEval20() { return this.firstEval20 + 1; },
set accessEval20(value) { setterSinkEval20 = value; },
};
check(orderLog[orderStart20] === 300, "order shorthand setup 20");
check(orderLog[orderStart20 + 1] === 300 + 2, "order computed name 20");
check(orderLog[orderStart20 + 2] === 300 + 6, "order first value 20");
check(orderLog[orderStart20 + 3] === 300 + 8, "order computed value 20");
check(orderLog[orderStart20 + 4] === 300 + 10, "order spread expression 20");
check(orderLog[orderStart20 + 5] === 300 + 11, "order override value 20");
check(evalObj20.firstEval20 === 300 + 7, "first data property 20");
check(evalObj20.shorthandValue20 === 300 + 1, "shorthand data property 20");
check(evalObj20[computedName20] === 300 + 9, "computed data property 20");
check(evalObj20.spreadEval20 === 300 + 3, "spread data property 20");
check(evalObj20.overrideEval20 === 300 + 12, "later property overrides spread 20");
check(Object.getPrototypeOf(evalObj20) === protoEval20, "literal proto setter 20");
check(evalObj20.inheritedEval20 === 300 + 5, "literal proto inherited 20");
check(evalObj20.methodEval20(2) === (300 + 7) + (300 + 12) + 2, "method definition evaluation 20");
check(evalObj20.accessEval20 === 300 + 8, "getter definition evaluation 20");
evalObj20.accessEval20 = 300 + 14;
check(setterSinkEval20 === 300 + 14, "setter definition evaluation 20");

var orderStart21 = orderLog.length;
var shorthandValue21 = mark(313, 313 + 1);
var computedName21 = mark(313 + 2, "computedEval21");
var spreadSource21 = { spreadEval21: 313 + 3, overrideEval21: 313 + 4 };
var protoEval21 = { inheritedEval21: 313 + 5 };
var setterSinkEval21 = 0;
var evalObj21 = {
firstEval21: mark(313 + 6, 313 + 7),
shorthandValue21,
[computedName21]: mark(313 + 8, 313 + 9),
...markObject(313 + 10, spreadSource21),
overrideEval21: mark(313 + 11, 313 + 12),
__proto__: protoEval21,
methodEval21(extra) { return this.firstEval21 + this.overrideEval21 + extra; },
get accessEval21() { return this.firstEval21 + 1; },
set accessEval21(value) { setterSinkEval21 = value; },
};
check(orderLog[orderStart21] === 313, "order shorthand setup 21");
check(orderLog[orderStart21 + 1] === 313 + 2, "order computed name 21");
check(orderLog[orderStart21 + 2] === 313 + 6, "order first value 21");
check(orderLog[orderStart21 + 3] === 313 + 8, "order computed value 21");
check(orderLog[orderStart21 + 4] === 313 + 10, "order spread expression 21");
check(orderLog[orderStart21 + 5] === 313 + 11, "order override value 21");
check(evalObj21.firstEval21 === 313 + 7, "first data property 21");
check(evalObj21.shorthandValue21 === 313 + 1, "shorthand data property 21");
check(evalObj21[computedName21] === 313 + 9, "computed data property 21");
check(evalObj21.spreadEval21 === 313 + 3, "spread data property 21");
check(evalObj21.overrideEval21 === 313 + 12, "later property overrides spread 21");
check(Object.getPrototypeOf(evalObj21) === protoEval21, "literal proto setter 21");
check(evalObj21.inheritedEval21 === 313 + 5, "literal proto inherited 21");
check(evalObj21.methodEval21(2) === (313 + 7) + (313 + 12) + 2, "method definition evaluation 21");
check(evalObj21.accessEval21 === 313 + 8, "getter definition evaluation 21");
evalObj21.accessEval21 = 313 + 14;
check(setterSinkEval21 === 313 + 14, "setter definition evaluation 21");

var orderStart22 = orderLog.length;
var shorthandValue22 = mark(326, 326 + 1);
var computedName22 = mark(326 + 2, "computedEval22");
var spreadSource22 = { spreadEval22: 326 + 3, overrideEval22: 326 + 4 };
var protoEval22 = { inheritedEval22: 326 + 5 };
var setterSinkEval22 = 0;
var evalObj22 = {
firstEval22: mark(326 + 6, 326 + 7),
shorthandValue22,
[computedName22]: mark(326 + 8, 326 + 9),
...markObject(326 + 10, spreadSource22),
overrideEval22: mark(326 + 11, 326 + 12),
__proto__: protoEval22,
methodEval22(extra) { return this.firstEval22 + this.overrideEval22 + extra; },
get accessEval22() { return this.firstEval22 + 1; },
set accessEval22(value) { setterSinkEval22 = value; },
};
check(orderLog[orderStart22] === 326, "order shorthand setup 22");
check(orderLog[orderStart22 + 1] === 326 + 2, "order computed name 22");
check(orderLog[orderStart22 + 2] === 326 + 6, "order first value 22");
check(orderLog[orderStart22 + 3] === 326 + 8, "order computed value 22");
check(orderLog[orderStart22 + 4] === 326 + 10, "order spread expression 22");
check(orderLog[orderStart22 + 5] === 326 + 11, "order override value 22");
check(evalObj22.firstEval22 === 326 + 7, "first data property 22");
check(evalObj22.shorthandValue22 === 326 + 1, "shorthand data property 22");
check(evalObj22[computedName22] === 326 + 9, "computed data property 22");
check(evalObj22.spreadEval22 === 326 + 3, "spread data property 22");
check(evalObj22.overrideEval22 === 326 + 12, "later property overrides spread 22");
check(Object.getPrototypeOf(evalObj22) === protoEval22, "literal proto setter 22");
check(evalObj22.inheritedEval22 === 326 + 5, "literal proto inherited 22");
check(evalObj22.methodEval22(2) === (326 + 7) + (326 + 12) + 2, "method definition evaluation 22");
check(evalObj22.accessEval22 === 326 + 8, "getter definition evaluation 22");
evalObj22.accessEval22 = 326 + 14;
check(setterSinkEval22 === 326 + 14, "setter definition evaluation 22");

var orderStart23 = orderLog.length;
var shorthandValue23 = mark(339, 339 + 1);
var computedName23 = mark(339 + 2, "computedEval23");
var spreadSource23 = { spreadEval23: 339 + 3, overrideEval23: 339 + 4 };
var protoEval23 = { inheritedEval23: 339 + 5 };
var setterSinkEval23 = 0;
var evalObj23 = {
firstEval23: mark(339 + 6, 339 + 7),
shorthandValue23,
[computedName23]: mark(339 + 8, 339 + 9),
...markObject(339 + 10, spreadSource23),
overrideEval23: mark(339 + 11, 339 + 12),
__proto__: protoEval23,
methodEval23(extra) { return this.firstEval23 + this.overrideEval23 + extra; },
get accessEval23() { return this.firstEval23 + 1; },
set accessEval23(value) { setterSinkEval23 = value; },
};
check(orderLog[orderStart23] === 339, "order shorthand setup 23");
check(orderLog[orderStart23 + 1] === 339 + 2, "order computed name 23");
check(orderLog[orderStart23 + 2] === 339 + 6, "order first value 23");
check(orderLog[orderStart23 + 3] === 339 + 8, "order computed value 23");
check(orderLog[orderStart23 + 4] === 339 + 10, "order spread expression 23");
check(orderLog[orderStart23 + 5] === 339 + 11, "order override value 23");
check(evalObj23.firstEval23 === 339 + 7, "first data property 23");
check(evalObj23.shorthandValue23 === 339 + 1, "shorthand data property 23");
check(evalObj23[computedName23] === 339 + 9, "computed data property 23");
check(evalObj23.spreadEval23 === 339 + 3, "spread data property 23");
check(evalObj23.overrideEval23 === 339 + 12, "later property overrides spread 23");
check(Object.getPrototypeOf(evalObj23) === protoEval23, "literal proto setter 23");
check(evalObj23.inheritedEval23 === 339 + 5, "literal proto inherited 23");
check(evalObj23.methodEval23(2) === (339 + 7) + (339 + 12) + 2, "method definition evaluation 23");
check(evalObj23.accessEval23 === 339 + 8, "getter definition evaluation 23");
evalObj23.accessEval23 = 339 + 14;
check(setterSinkEval23 === 339 + 14, "setter definition evaluation 23");

var orderStart24 = orderLog.length;
var shorthandValue24 = mark(352, 352 + 1);
var computedName24 = mark(352 + 2, "computedEval24");
var spreadSource24 = { spreadEval24: 352 + 3, overrideEval24: 352 + 4 };
var protoEval24 = { inheritedEval24: 352 + 5 };
var setterSinkEval24 = 0;
var evalObj24 = {
firstEval24: mark(352 + 6, 352 + 7),
shorthandValue24,
[computedName24]: mark(352 + 8, 352 + 9),
...markObject(352 + 10, spreadSource24),
overrideEval24: mark(352 + 11, 352 + 12),
__proto__: protoEval24,
methodEval24(extra) { return this.firstEval24 + this.overrideEval24 + extra; },
get accessEval24() { return this.firstEval24 + 1; },
set accessEval24(value) { setterSinkEval24 = value; },
};
check(orderLog[orderStart24] === 352, "order shorthand setup 24");
check(orderLog[orderStart24 + 1] === 352 + 2, "order computed name 24");
check(orderLog[orderStart24 + 2] === 352 + 6, "order first value 24");
check(orderLog[orderStart24 + 3] === 352 + 8, "order computed value 24");
check(orderLog[orderStart24 + 4] === 352 + 10, "order spread expression 24");
check(orderLog[orderStart24 + 5] === 352 + 11, "order override value 24");
check(evalObj24.firstEval24 === 352 + 7, "first data property 24");
check(evalObj24.shorthandValue24 === 352 + 1, "shorthand data property 24");
check(evalObj24[computedName24] === 352 + 9, "computed data property 24");
check(evalObj24.spreadEval24 === 352 + 3, "spread data property 24");
check(evalObj24.overrideEval24 === 352 + 12, "later property overrides spread 24");
check(Object.getPrototypeOf(evalObj24) === protoEval24, "literal proto setter 24");
check(evalObj24.inheritedEval24 === 352 + 5, "literal proto inherited 24");
check(evalObj24.methodEval24(2) === (352 + 7) + (352 + 12) + 2, "method definition evaluation 24");
check(evalObj24.accessEval24 === 352 + 8, "getter definition evaluation 24");
evalObj24.accessEval24 = 352 + 14;
check(setterSinkEval24 === 352 + 14, "setter definition evaluation 24");

var orderStart25 = orderLog.length;
var shorthandValue25 = mark(365, 365 + 1);
var computedName25 = mark(365 + 2, "computedEval25");
var spreadSource25 = { spreadEval25: 365 + 3, overrideEval25: 365 + 4 };
var protoEval25 = { inheritedEval25: 365 + 5 };
var setterSinkEval25 = 0;
var evalObj25 = {
firstEval25: mark(365 + 6, 365 + 7),
shorthandValue25,
[computedName25]: mark(365 + 8, 365 + 9),
...markObject(365 + 10, spreadSource25),
overrideEval25: mark(365 + 11, 365 + 12),
__proto__: protoEval25,
methodEval25(extra) { return this.firstEval25 + this.overrideEval25 + extra; },
get accessEval25() { return this.firstEval25 + 1; },
set accessEval25(value) { setterSinkEval25 = value; },
};
check(orderLog[orderStart25] === 365, "order shorthand setup 25");
check(orderLog[orderStart25 + 1] === 365 + 2, "order computed name 25");
check(orderLog[orderStart25 + 2] === 365 + 6, "order first value 25");
check(orderLog[orderStart25 + 3] === 365 + 8, "order computed value 25");
check(orderLog[orderStart25 + 4] === 365 + 10, "order spread expression 25");
check(orderLog[orderStart25 + 5] === 365 + 11, "order override value 25");
check(evalObj25.firstEval25 === 365 + 7, "first data property 25");
check(evalObj25.shorthandValue25 === 365 + 1, "shorthand data property 25");
check(evalObj25[computedName25] === 365 + 9, "computed data property 25");
check(evalObj25.spreadEval25 === 365 + 3, "spread data property 25");
check(evalObj25.overrideEval25 === 365 + 12, "later property overrides spread 25");
check(Object.getPrototypeOf(evalObj25) === protoEval25, "literal proto setter 25");
check(evalObj25.inheritedEval25 === 365 + 5, "literal proto inherited 25");
check(evalObj25.methodEval25(2) === (365 + 7) + (365 + 12) + 2, "method definition evaluation 25");
check(evalObj25.accessEval25 === 365 + 8, "getter definition evaluation 25");
evalObj25.accessEval25 = 365 + 14;
check(setterSinkEval25 === 365 + 14, "setter definition evaluation 25");

var orderStart26 = orderLog.length;
var shorthandValue26 = mark(378, 378 + 1);
var computedName26 = mark(378 + 2, "computedEval26");
var spreadSource26 = { spreadEval26: 378 + 3, overrideEval26: 378 + 4 };
var protoEval26 = { inheritedEval26: 378 + 5 };
var setterSinkEval26 = 0;
var evalObj26 = {
firstEval26: mark(378 + 6, 378 + 7),
shorthandValue26,
[computedName26]: mark(378 + 8, 378 + 9),
...markObject(378 + 10, spreadSource26),
overrideEval26: mark(378 + 11, 378 + 12),
__proto__: protoEval26,
methodEval26(extra) { return this.firstEval26 + this.overrideEval26 + extra; },
get accessEval26() { return this.firstEval26 + 1; },
set accessEval26(value) { setterSinkEval26 = value; },
};
check(orderLog[orderStart26] === 378, "order shorthand setup 26");
check(orderLog[orderStart26 + 1] === 378 + 2, "order computed name 26");
check(orderLog[orderStart26 + 2] === 378 + 6, "order first value 26");
check(orderLog[orderStart26 + 3] === 378 + 8, "order computed value 26");
check(orderLog[orderStart26 + 4] === 378 + 10, "order spread expression 26");
check(orderLog[orderStart26 + 5] === 378 + 11, "order override value 26");
check(evalObj26.firstEval26 === 378 + 7, "first data property 26");
check(evalObj26.shorthandValue26 === 378 + 1, "shorthand data property 26");
check(evalObj26[computedName26] === 378 + 9, "computed data property 26");
check(evalObj26.spreadEval26 === 378 + 3, "spread data property 26");
check(evalObj26.overrideEval26 === 378 + 12, "later property overrides spread 26");
check(Object.getPrototypeOf(evalObj26) === protoEval26, "literal proto setter 26");
check(evalObj26.inheritedEval26 === 378 + 5, "literal proto inherited 26");
check(evalObj26.methodEval26(2) === (378 + 7) + (378 + 12) + 2, "method definition evaluation 26");
check(evalObj26.accessEval26 === 378 + 8, "getter definition evaluation 26");
evalObj26.accessEval26 = 378 + 14;
check(setterSinkEval26 === 378 + 14, "setter definition evaluation 26");

var orderStart27 = orderLog.length;
var shorthandValue27 = mark(391, 391 + 1);
var computedName27 = mark(391 + 2, "computedEval27");
var spreadSource27 = { spreadEval27: 391 + 3, overrideEval27: 391 + 4 };
var protoEval27 = { inheritedEval27: 391 + 5 };
var setterSinkEval27 = 0;
var evalObj27 = {
firstEval27: mark(391 + 6, 391 + 7),
shorthandValue27,
[computedName27]: mark(391 + 8, 391 + 9),
...markObject(391 + 10, spreadSource27),
overrideEval27: mark(391 + 11, 391 + 12),
__proto__: protoEval27,
methodEval27(extra) { return this.firstEval27 + this.overrideEval27 + extra; },
get accessEval27() { return this.firstEval27 + 1; },
set accessEval27(value) { setterSinkEval27 = value; },
};
check(orderLog[orderStart27] === 391, "order shorthand setup 27");
check(orderLog[orderStart27 + 1] === 391 + 2, "order computed name 27");
check(orderLog[orderStart27 + 2] === 391 + 6, "order first value 27");
check(orderLog[orderStart27 + 3] === 391 + 8, "order computed value 27");
check(orderLog[orderStart27 + 4] === 391 + 10, "order spread expression 27");
check(orderLog[orderStart27 + 5] === 391 + 11, "order override value 27");
check(evalObj27.firstEval27 === 391 + 7, "first data property 27");
check(evalObj27.shorthandValue27 === 391 + 1, "shorthand data property 27");
check(evalObj27[computedName27] === 391 + 9, "computed data property 27");
check(evalObj27.spreadEval27 === 391 + 3, "spread data property 27");
check(evalObj27.overrideEval27 === 391 + 12, "later property overrides spread 27");
check(Object.getPrototypeOf(evalObj27) === protoEval27, "literal proto setter 27");
check(evalObj27.inheritedEval27 === 391 + 5, "literal proto inherited 27");
check(evalObj27.methodEval27(2) === (391 + 7) + (391 + 12) + 2, "method definition evaluation 27");
check(evalObj27.accessEval27 === 391 + 8, "getter definition evaluation 27");
evalObj27.accessEval27 = 391 + 14;
check(setterSinkEval27 === 391 + 14, "setter definition evaluation 27");

var orderStart28 = orderLog.length;
var shorthandValue28 = mark(404, 404 + 1);
var computedName28 = mark(404 + 2, "computedEval28");
var spreadSource28 = { spreadEval28: 404 + 3, overrideEval28: 404 + 4 };
var protoEval28 = { inheritedEval28: 404 + 5 };
var setterSinkEval28 = 0;
var evalObj28 = {
firstEval28: mark(404 + 6, 404 + 7),
shorthandValue28,
[computedName28]: mark(404 + 8, 404 + 9),
...markObject(404 + 10, spreadSource28),
overrideEval28: mark(404 + 11, 404 + 12),
__proto__: protoEval28,
methodEval28(extra) { return this.firstEval28 + this.overrideEval28 + extra; },
get accessEval28() { return this.firstEval28 + 1; },
set accessEval28(value) { setterSinkEval28 = value; },
};
check(orderLog[orderStart28] === 404, "order shorthand setup 28");
check(orderLog[orderStart28 + 1] === 404 + 2, "order computed name 28");
check(orderLog[orderStart28 + 2] === 404 + 6, "order first value 28");
check(orderLog[orderStart28 + 3] === 404 + 8, "order computed value 28");
check(orderLog[orderStart28 + 4] === 404 + 10, "order spread expression 28");
check(orderLog[orderStart28 + 5] === 404 + 11, "order override value 28");
check(evalObj28.firstEval28 === 404 + 7, "first data property 28");
check(evalObj28.shorthandValue28 === 404 + 1, "shorthand data property 28");
check(evalObj28[computedName28] === 404 + 9, "computed data property 28");
check(evalObj28.spreadEval28 === 404 + 3, "spread data property 28");
check(evalObj28.overrideEval28 === 404 + 12, "later property overrides spread 28");
check(Object.getPrototypeOf(evalObj28) === protoEval28, "literal proto setter 28");
check(evalObj28.inheritedEval28 === 404 + 5, "literal proto inherited 28");
check(evalObj28.methodEval28(2) === (404 + 7) + (404 + 12) + 2, "method definition evaluation 28");
check(evalObj28.accessEval28 === 404 + 8, "getter definition evaluation 28");
evalObj28.accessEval28 = 404 + 14;
check(setterSinkEval28 === 404 + 14, "setter definition evaluation 28");

var orderStart29 = orderLog.length;
var shorthandValue29 = mark(417, 417 + 1);
var computedName29 = mark(417 + 2, "computedEval29");
var spreadSource29 = { spreadEval29: 417 + 3, overrideEval29: 417 + 4 };
var protoEval29 = { inheritedEval29: 417 + 5 };
var setterSinkEval29 = 0;
var evalObj29 = {
firstEval29: mark(417 + 6, 417 + 7),
shorthandValue29,
[computedName29]: mark(417 + 8, 417 + 9),
...markObject(417 + 10, spreadSource29),
overrideEval29: mark(417 + 11, 417 + 12),
__proto__: protoEval29,
methodEval29(extra) { return this.firstEval29 + this.overrideEval29 + extra; },
get accessEval29() { return this.firstEval29 + 1; },
set accessEval29(value) { setterSinkEval29 = value; },
};
check(orderLog[orderStart29] === 417, "order shorthand setup 29");
check(orderLog[orderStart29 + 1] === 417 + 2, "order computed name 29");
check(orderLog[orderStart29 + 2] === 417 + 6, "order first value 29");
check(orderLog[orderStart29 + 3] === 417 + 8, "order computed value 29");
check(orderLog[orderStart29 + 4] === 417 + 10, "order spread expression 29");
check(orderLog[orderStart29 + 5] === 417 + 11, "order override value 29");
check(evalObj29.firstEval29 === 417 + 7, "first data property 29");
check(evalObj29.shorthandValue29 === 417 + 1, "shorthand data property 29");
check(evalObj29[computedName29] === 417 + 9, "computed data property 29");
check(evalObj29.spreadEval29 === 417 + 3, "spread data property 29");
check(evalObj29.overrideEval29 === 417 + 12, "later property overrides spread 29");
check(Object.getPrototypeOf(evalObj29) === protoEval29, "literal proto setter 29");
check(evalObj29.inheritedEval29 === 417 + 5, "literal proto inherited 29");
check(evalObj29.methodEval29(2) === (417 + 7) + (417 + 12) + 2, "method definition evaluation 29");
check(evalObj29.accessEval29 === 417 + 8, "getter definition evaluation 29");
evalObj29.accessEval29 = 417 + 14;
check(setterSinkEval29 === 417 + 14, "setter definition evaluation 29");

var orderStart30 = orderLog.length;
var shorthandValue30 = mark(430, 430 + 1);
var computedName30 = mark(430 + 2, "computedEval30");
var spreadSource30 = { spreadEval30: 430 + 3, overrideEval30: 430 + 4 };
var protoEval30 = { inheritedEval30: 430 + 5 };
var setterSinkEval30 = 0;
var evalObj30 = {
firstEval30: mark(430 + 6, 430 + 7),
shorthandValue30,
[computedName30]: mark(430 + 8, 430 + 9),
...markObject(430 + 10, spreadSource30),
overrideEval30: mark(430 + 11, 430 + 12),
__proto__: protoEval30,
methodEval30(extra) { return this.firstEval30 + this.overrideEval30 + extra; },
get accessEval30() { return this.firstEval30 + 1; },
set accessEval30(value) { setterSinkEval30 = value; },
};
check(orderLog[orderStart30] === 430, "order shorthand setup 30");
check(orderLog[orderStart30 + 1] === 430 + 2, "order computed name 30");
check(orderLog[orderStart30 + 2] === 430 + 6, "order first value 30");
check(orderLog[orderStart30 + 3] === 430 + 8, "order computed value 30");
check(orderLog[orderStart30 + 4] === 430 + 10, "order spread expression 30");
check(orderLog[orderStart30 + 5] === 430 + 11, "order override value 30");
check(evalObj30.firstEval30 === 430 + 7, "first data property 30");
check(evalObj30.shorthandValue30 === 430 + 1, "shorthand data property 30");
check(evalObj30[computedName30] === 430 + 9, "computed data property 30");
check(evalObj30.spreadEval30 === 430 + 3, "spread data property 30");
check(evalObj30.overrideEval30 === 430 + 12, "later property overrides spread 30");
check(Object.getPrototypeOf(evalObj30) === protoEval30, "literal proto setter 30");
check(evalObj30.inheritedEval30 === 430 + 5, "literal proto inherited 30");
check(evalObj30.methodEval30(2) === (430 + 7) + (430 + 12) + 2, "method definition evaluation 30");
check(evalObj30.accessEval30 === 430 + 8, "getter definition evaluation 30");
evalObj30.accessEval30 = 430 + 14;
check(setterSinkEval30 === 430 + 14, "setter definition evaluation 30");

var orderStart31 = orderLog.length;
var shorthandValue31 = mark(443, 443 + 1);
var computedName31 = mark(443 + 2, "computedEval31");
var spreadSource31 = { spreadEval31: 443 + 3, overrideEval31: 443 + 4 };
var protoEval31 = { inheritedEval31: 443 + 5 };
var setterSinkEval31 = 0;
var evalObj31 = {
firstEval31: mark(443 + 6, 443 + 7),
shorthandValue31,
[computedName31]: mark(443 + 8, 443 + 9),
...markObject(443 + 10, spreadSource31),
overrideEval31: mark(443 + 11, 443 + 12),
__proto__: protoEval31,
methodEval31(extra) { return this.firstEval31 + this.overrideEval31 + extra; },
get accessEval31() { return this.firstEval31 + 1; },
set accessEval31(value) { setterSinkEval31 = value; },
};
check(orderLog[orderStart31] === 443, "order shorthand setup 31");
check(orderLog[orderStart31 + 1] === 443 + 2, "order computed name 31");
check(orderLog[orderStart31 + 2] === 443 + 6, "order first value 31");
check(orderLog[orderStart31 + 3] === 443 + 8, "order computed value 31");
check(orderLog[orderStart31 + 4] === 443 + 10, "order spread expression 31");
check(orderLog[orderStart31 + 5] === 443 + 11, "order override value 31");
check(evalObj31.firstEval31 === 443 + 7, "first data property 31");
check(evalObj31.shorthandValue31 === 443 + 1, "shorthand data property 31");
check(evalObj31[computedName31] === 443 + 9, "computed data property 31");
check(evalObj31.spreadEval31 === 443 + 3, "spread data property 31");
check(evalObj31.overrideEval31 === 443 + 12, "later property overrides spread 31");
check(Object.getPrototypeOf(evalObj31) === protoEval31, "literal proto setter 31");
check(evalObj31.inheritedEval31 === 443 + 5, "literal proto inherited 31");
check(evalObj31.methodEval31(2) === (443 + 7) + (443 + 12) + 2, "method definition evaluation 31");
check(evalObj31.accessEval31 === 443 + 8, "getter definition evaluation 31");
evalObj31.accessEval31 = 443 + 14;
check(setterSinkEval31 === 443 + 14, "setter definition evaluation 31");

var orderStart32 = orderLog.length;
var shorthandValue32 = mark(456, 456 + 1);
var computedName32 = mark(456 + 2, "computedEval32");
var spreadSource32 = { spreadEval32: 456 + 3, overrideEval32: 456 + 4 };
var protoEval32 = { inheritedEval32: 456 + 5 };
var setterSinkEval32 = 0;
var evalObj32 = {
firstEval32: mark(456 + 6, 456 + 7),
shorthandValue32,
[computedName32]: mark(456 + 8, 456 + 9),
...markObject(456 + 10, spreadSource32),
overrideEval32: mark(456 + 11, 456 + 12),
__proto__: protoEval32,
methodEval32(extra) { return this.firstEval32 + this.overrideEval32 + extra; },
get accessEval32() { return this.firstEval32 + 1; },
set accessEval32(value) { setterSinkEval32 = value; },
};
check(orderLog[orderStart32] === 456, "order shorthand setup 32");
check(orderLog[orderStart32 + 1] === 456 + 2, "order computed name 32");
check(orderLog[orderStart32 + 2] === 456 + 6, "order first value 32");
check(orderLog[orderStart32 + 3] === 456 + 8, "order computed value 32");
check(orderLog[orderStart32 + 4] === 456 + 10, "order spread expression 32");
check(orderLog[orderStart32 + 5] === 456 + 11, "order override value 32");
check(evalObj32.firstEval32 === 456 + 7, "first data property 32");
check(evalObj32.shorthandValue32 === 456 + 1, "shorthand data property 32");
check(evalObj32[computedName32] === 456 + 9, "computed data property 32");
check(evalObj32.spreadEval32 === 456 + 3, "spread data property 32");
check(evalObj32.overrideEval32 === 456 + 12, "later property overrides spread 32");
check(Object.getPrototypeOf(evalObj32) === protoEval32, "literal proto setter 32");
check(evalObj32.inheritedEval32 === 456 + 5, "literal proto inherited 32");
check(evalObj32.methodEval32(2) === (456 + 7) + (456 + 12) + 2, "method definition evaluation 32");
check(evalObj32.accessEval32 === 456 + 8, "getter definition evaluation 32");
evalObj32.accessEval32 = 456 + 14;
check(setterSinkEval32 === 456 + 14, "setter definition evaluation 32");

var orderStart33 = orderLog.length;
var shorthandValue33 = mark(469, 469 + 1);
var computedName33 = mark(469 + 2, "computedEval33");
var spreadSource33 = { spreadEval33: 469 + 3, overrideEval33: 469 + 4 };
var protoEval33 = { inheritedEval33: 469 + 5 };
var setterSinkEval33 = 0;
var evalObj33 = {
firstEval33: mark(469 + 6, 469 + 7),
shorthandValue33,
[computedName33]: mark(469 + 8, 469 + 9),
...markObject(469 + 10, spreadSource33),
overrideEval33: mark(469 + 11, 469 + 12),
__proto__: protoEval33,
methodEval33(extra) { return this.firstEval33 + this.overrideEval33 + extra; },
get accessEval33() { return this.firstEval33 + 1; },
set accessEval33(value) { setterSinkEval33 = value; },
};
check(orderLog[orderStart33] === 469, "order shorthand setup 33");
check(orderLog[orderStart33 + 1] === 469 + 2, "order computed name 33");
check(orderLog[orderStart33 + 2] === 469 + 6, "order first value 33");
check(orderLog[orderStart33 + 3] === 469 + 8, "order computed value 33");
check(orderLog[orderStart33 + 4] === 469 + 10, "order spread expression 33");
check(orderLog[orderStart33 + 5] === 469 + 11, "order override value 33");
check(evalObj33.firstEval33 === 469 + 7, "first data property 33");
check(evalObj33.shorthandValue33 === 469 + 1, "shorthand data property 33");
check(evalObj33[computedName33] === 469 + 9, "computed data property 33");
check(evalObj33.spreadEval33 === 469 + 3, "spread data property 33");
check(evalObj33.overrideEval33 === 469 + 12, "later property overrides spread 33");
check(Object.getPrototypeOf(evalObj33) === protoEval33, "literal proto setter 33");
check(evalObj33.inheritedEval33 === 469 + 5, "literal proto inherited 33");
check(evalObj33.methodEval33(2) === (469 + 7) + (469 + 12) + 2, "method definition evaluation 33");
check(evalObj33.accessEval33 === 469 + 8, "getter definition evaluation 33");
evalObj33.accessEval33 = 469 + 14;
check(setterSinkEval33 === 469 + 14, "setter definition evaluation 33");

var orderStart34 = orderLog.length;
var shorthandValue34 = mark(482, 482 + 1);
var computedName34 = mark(482 + 2, "computedEval34");
var spreadSource34 = { spreadEval34: 482 + 3, overrideEval34: 482 + 4 };
var protoEval34 = { inheritedEval34: 482 + 5 };
var setterSinkEval34 = 0;
var evalObj34 = {
firstEval34: mark(482 + 6, 482 + 7),
shorthandValue34,
[computedName34]: mark(482 + 8, 482 + 9),
...markObject(482 + 10, spreadSource34),
overrideEval34: mark(482 + 11, 482 + 12),
__proto__: protoEval34,
methodEval34(extra) { return this.firstEval34 + this.overrideEval34 + extra; },
get accessEval34() { return this.firstEval34 + 1; },
set accessEval34(value) { setterSinkEval34 = value; },
};
check(orderLog[orderStart34] === 482, "order shorthand setup 34");
check(orderLog[orderStart34 + 1] === 482 + 2, "order computed name 34");
check(orderLog[orderStart34 + 2] === 482 + 6, "order first value 34");
check(orderLog[orderStart34 + 3] === 482 + 8, "order computed value 34");
check(orderLog[orderStart34 + 4] === 482 + 10, "order spread expression 34");
check(orderLog[orderStart34 + 5] === 482 + 11, "order override value 34");
check(evalObj34.firstEval34 === 482 + 7, "first data property 34");
check(evalObj34.shorthandValue34 === 482 + 1, "shorthand data property 34");
check(evalObj34[computedName34] === 482 + 9, "computed data property 34");
check(evalObj34.spreadEval34 === 482 + 3, "spread data property 34");
check(evalObj34.overrideEval34 === 482 + 12, "later property overrides spread 34");
check(Object.getPrototypeOf(evalObj34) === protoEval34, "literal proto setter 34");
check(evalObj34.inheritedEval34 === 482 + 5, "literal proto inherited 34");
check(evalObj34.methodEval34(2) === (482 + 7) + (482 + 12) + 2, "method definition evaluation 34");
check(evalObj34.accessEval34 === 482 + 8, "getter definition evaluation 34");
evalObj34.accessEval34 = 482 + 14;
check(setterSinkEval34 === 482 + 14, "setter definition evaluation 34");

var orderStart35 = orderLog.length;
var shorthandValue35 = mark(495, 495 + 1);
var computedName35 = mark(495 + 2, "computedEval35");
var spreadSource35 = { spreadEval35: 495 + 3, overrideEval35: 495 + 4 };
var protoEval35 = { inheritedEval35: 495 + 5 };
var setterSinkEval35 = 0;
var evalObj35 = {
firstEval35: mark(495 + 6, 495 + 7),
shorthandValue35,
[computedName35]: mark(495 + 8, 495 + 9),
...markObject(495 + 10, spreadSource35),
overrideEval35: mark(495 + 11, 495 + 12),
__proto__: protoEval35,
methodEval35(extra) { return this.firstEval35 + this.overrideEval35 + extra; },
get accessEval35() { return this.firstEval35 + 1; },
set accessEval35(value) { setterSinkEval35 = value; },
};
check(orderLog[orderStart35] === 495, "order shorthand setup 35");
check(orderLog[orderStart35 + 1] === 495 + 2, "order computed name 35");
check(orderLog[orderStart35 + 2] === 495 + 6, "order first value 35");
check(orderLog[orderStart35 + 3] === 495 + 8, "order computed value 35");
check(orderLog[orderStart35 + 4] === 495 + 10, "order spread expression 35");
check(orderLog[orderStart35 + 5] === 495 + 11, "order override value 35");
check(evalObj35.firstEval35 === 495 + 7, "first data property 35");
check(evalObj35.shorthandValue35 === 495 + 1, "shorthand data property 35");
check(evalObj35[computedName35] === 495 + 9, "computed data property 35");
check(evalObj35.spreadEval35 === 495 + 3, "spread data property 35");
check(evalObj35.overrideEval35 === 495 + 12, "later property overrides spread 35");
check(Object.getPrototypeOf(evalObj35) === protoEval35, "literal proto setter 35");
check(evalObj35.inheritedEval35 === 495 + 5, "literal proto inherited 35");
check(evalObj35.methodEval35(2) === (495 + 7) + (495 + 12) + 2, "method definition evaluation 35");
check(evalObj35.accessEval35 === 495 + 8, "getter definition evaluation 35");
evalObj35.accessEval35 = 495 + 14;
check(setterSinkEval35 === 495 + 14, "setter definition evaluation 35");

var orderStart36 = orderLog.length;
var shorthandValue36 = mark(508, 508 + 1);
var computedName36 = mark(508 + 2, "computedEval36");
var spreadSource36 = { spreadEval36: 508 + 3, overrideEval36: 508 + 4 };
var protoEval36 = { inheritedEval36: 508 + 5 };
var setterSinkEval36 = 0;
var evalObj36 = {
firstEval36: mark(508 + 6, 508 + 7),
shorthandValue36,
[computedName36]: mark(508 + 8, 508 + 9),
...markObject(508 + 10, spreadSource36),
overrideEval36: mark(508 + 11, 508 + 12),
__proto__: protoEval36,
methodEval36(extra) { return this.firstEval36 + this.overrideEval36 + extra; },
get accessEval36() { return this.firstEval36 + 1; },
set accessEval36(value) { setterSinkEval36 = value; },
};
check(orderLog[orderStart36] === 508, "order shorthand setup 36");
check(orderLog[orderStart36 + 1] === 508 + 2, "order computed name 36");
check(orderLog[orderStart36 + 2] === 508 + 6, "order first value 36");
check(orderLog[orderStart36 + 3] === 508 + 8, "order computed value 36");
check(orderLog[orderStart36 + 4] === 508 + 10, "order spread expression 36");
check(orderLog[orderStart36 + 5] === 508 + 11, "order override value 36");
check(evalObj36.firstEval36 === 508 + 7, "first data property 36");
check(evalObj36.shorthandValue36 === 508 + 1, "shorthand data property 36");
check(evalObj36[computedName36] === 508 + 9, "computed data property 36");
check(evalObj36.spreadEval36 === 508 + 3, "spread data property 36");
check(evalObj36.overrideEval36 === 508 + 12, "later property overrides spread 36");
check(Object.getPrototypeOf(evalObj36) === protoEval36, "literal proto setter 36");
check(evalObj36.inheritedEval36 === 508 + 5, "literal proto inherited 36");
check(evalObj36.methodEval36(2) === (508 + 7) + (508 + 12) + 2, "method definition evaluation 36");
check(evalObj36.accessEval36 === 508 + 8, "getter definition evaluation 36");
evalObj36.accessEval36 = 508 + 14;
check(setterSinkEval36 === 508 + 14, "setter definition evaluation 36");

var orderStart37 = orderLog.length;
var shorthandValue37 = mark(521, 521 + 1);
var computedName37 = mark(521 + 2, "computedEval37");
var spreadSource37 = { spreadEval37: 521 + 3, overrideEval37: 521 + 4 };
var protoEval37 = { inheritedEval37: 521 + 5 };
var setterSinkEval37 = 0;
var evalObj37 = {
firstEval37: mark(521 + 6, 521 + 7),
shorthandValue37,
[computedName37]: mark(521 + 8, 521 + 9),
...markObject(521 + 10, spreadSource37),
overrideEval37: mark(521 + 11, 521 + 12),
__proto__: protoEval37,
methodEval37(extra) { return this.firstEval37 + this.overrideEval37 + extra; },
get accessEval37() { return this.firstEval37 + 1; },
set accessEval37(value) { setterSinkEval37 = value; },
};
check(orderLog[orderStart37] === 521, "order shorthand setup 37");
check(orderLog[orderStart37 + 1] === 521 + 2, "order computed name 37");
check(orderLog[orderStart37 + 2] === 521 + 6, "order first value 37");
check(orderLog[orderStart37 + 3] === 521 + 8, "order computed value 37");
check(orderLog[orderStart37 + 4] === 521 + 10, "order spread expression 37");
check(orderLog[orderStart37 + 5] === 521 + 11, "order override value 37");
check(evalObj37.firstEval37 === 521 + 7, "first data property 37");
check(evalObj37.shorthandValue37 === 521 + 1, "shorthand data property 37");
check(evalObj37[computedName37] === 521 + 9, "computed data property 37");
check(evalObj37.spreadEval37 === 521 + 3, "spread data property 37");
check(evalObj37.overrideEval37 === 521 + 12, "later property overrides spread 37");
check(Object.getPrototypeOf(evalObj37) === protoEval37, "literal proto setter 37");
check(evalObj37.inheritedEval37 === 521 + 5, "literal proto inherited 37");
check(evalObj37.methodEval37(2) === (521 + 7) + (521 + 12) + 2, "method definition evaluation 37");
check(evalObj37.accessEval37 === 521 + 8, "getter definition evaluation 37");
evalObj37.accessEval37 = 521 + 14;
check(setterSinkEval37 === 521 + 14, "setter definition evaluation 37");

var orderStart38 = orderLog.length;
var shorthandValue38 = mark(534, 534 + 1);
var computedName38 = mark(534 + 2, "computedEval38");
var spreadSource38 = { spreadEval38: 534 + 3, overrideEval38: 534 + 4 };
var protoEval38 = { inheritedEval38: 534 + 5 };
var setterSinkEval38 = 0;
var evalObj38 = {
firstEval38: mark(534 + 6, 534 + 7),
shorthandValue38,
[computedName38]: mark(534 + 8, 534 + 9),
...markObject(534 + 10, spreadSource38),
overrideEval38: mark(534 + 11, 534 + 12),
__proto__: protoEval38,
methodEval38(extra) { return this.firstEval38 + this.overrideEval38 + extra; },
get accessEval38() { return this.firstEval38 + 1; },
set accessEval38(value) { setterSinkEval38 = value; },
};
check(orderLog[orderStart38] === 534, "order shorthand setup 38");
check(orderLog[orderStart38 + 1] === 534 + 2, "order computed name 38");
check(orderLog[orderStart38 + 2] === 534 + 6, "order first value 38");
check(orderLog[orderStart38 + 3] === 534 + 8, "order computed value 38");
check(orderLog[orderStart38 + 4] === 534 + 10, "order spread expression 38");
check(orderLog[orderStart38 + 5] === 534 + 11, "order override value 38");
check(evalObj38.firstEval38 === 534 + 7, "first data property 38");
check(evalObj38.shorthandValue38 === 534 + 1, "shorthand data property 38");
check(evalObj38[computedName38] === 534 + 9, "computed data property 38");
check(evalObj38.spreadEval38 === 534 + 3, "spread data property 38");
check(evalObj38.overrideEval38 === 534 + 12, "later property overrides spread 38");
check(Object.getPrototypeOf(evalObj38) === protoEval38, "literal proto setter 38");
check(evalObj38.inheritedEval38 === 534 + 5, "literal proto inherited 38");
check(evalObj38.methodEval38(2) === (534 + 7) + (534 + 12) + 2, "method definition evaluation 38");
check(evalObj38.accessEval38 === 534 + 8, "getter definition evaluation 38");
evalObj38.accessEval38 = 534 + 14;
check(setterSinkEval38 === 534 + 14, "setter definition evaluation 38");

var orderStart39 = orderLog.length;
var shorthandValue39 = mark(547, 547 + 1);
var computedName39 = mark(547 + 2, "computedEval39");
var spreadSource39 = { spreadEval39: 547 + 3, overrideEval39: 547 + 4 };
var protoEval39 = { inheritedEval39: 547 + 5 };
var setterSinkEval39 = 0;
var evalObj39 = {
firstEval39: mark(547 + 6, 547 + 7),
shorthandValue39,
[computedName39]: mark(547 + 8, 547 + 9),
...markObject(547 + 10, spreadSource39),
overrideEval39: mark(547 + 11, 547 + 12),
__proto__: protoEval39,
methodEval39(extra) { return this.firstEval39 + this.overrideEval39 + extra; },
get accessEval39() { return this.firstEval39 + 1; },
set accessEval39(value) { setterSinkEval39 = value; },
};
check(orderLog[orderStart39] === 547, "order shorthand setup 39");
check(orderLog[orderStart39 + 1] === 547 + 2, "order computed name 39");
check(orderLog[orderStart39 + 2] === 547 + 6, "order first value 39");
check(orderLog[orderStart39 + 3] === 547 + 8, "order computed value 39");
check(orderLog[orderStart39 + 4] === 547 + 10, "order spread expression 39");
check(orderLog[orderStart39 + 5] === 547 + 11, "order override value 39");
check(evalObj39.firstEval39 === 547 + 7, "first data property 39");
check(evalObj39.shorthandValue39 === 547 + 1, "shorthand data property 39");
check(evalObj39[computedName39] === 547 + 9, "computed data property 39");
check(evalObj39.spreadEval39 === 547 + 3, "spread data property 39");
check(evalObj39.overrideEval39 === 547 + 12, "later property overrides spread 39");
check(Object.getPrototypeOf(evalObj39) === protoEval39, "literal proto setter 39");
check(evalObj39.inheritedEval39 === 547 + 5, "literal proto inherited 39");
check(evalObj39.methodEval39(2) === (547 + 7) + (547 + 12) + 2, "method definition evaluation 39");
check(evalObj39.accessEval39 === 547 + 8, "getter definition evaluation 39");
evalObj39.accessEval39 = 547 + 14;
check(setterSinkEval39 === 547 + 14, "setter definition evaluation 39");

var orderStart40 = orderLog.length;
var shorthandValue40 = mark(560, 560 + 1);
var computedName40 = mark(560 + 2, "computedEval40");
var spreadSource40 = { spreadEval40: 560 + 3, overrideEval40: 560 + 4 };
var protoEval40 = { inheritedEval40: 560 + 5 };
var setterSinkEval40 = 0;
var evalObj40 = {
firstEval40: mark(560 + 6, 560 + 7),
shorthandValue40,
[computedName40]: mark(560 + 8, 560 + 9),
...markObject(560 + 10, spreadSource40),
overrideEval40: mark(560 + 11, 560 + 12),
__proto__: protoEval40,
methodEval40(extra) { return this.firstEval40 + this.overrideEval40 + extra; },
get accessEval40() { return this.firstEval40 + 1; },
set accessEval40(value) { setterSinkEval40 = value; },
};
check(orderLog[orderStart40] === 560, "order shorthand setup 40");
check(orderLog[orderStart40 + 1] === 560 + 2, "order computed name 40");
check(orderLog[orderStart40 + 2] === 560 + 6, "order first value 40");
check(orderLog[orderStart40 + 3] === 560 + 8, "order computed value 40");
check(orderLog[orderStart40 + 4] === 560 + 10, "order spread expression 40");
check(orderLog[orderStart40 + 5] === 560 + 11, "order override value 40");
check(evalObj40.firstEval40 === 560 + 7, "first data property 40");
check(evalObj40.shorthandValue40 === 560 + 1, "shorthand data property 40");
check(evalObj40[computedName40] === 560 + 9, "computed data property 40");
check(evalObj40.spreadEval40 === 560 + 3, "spread data property 40");
check(evalObj40.overrideEval40 === 560 + 12, "later property overrides spread 40");
check(Object.getPrototypeOf(evalObj40) === protoEval40, "literal proto setter 40");
check(evalObj40.inheritedEval40 === 560 + 5, "literal proto inherited 40");
check(evalObj40.methodEval40(2) === (560 + 7) + (560 + 12) + 2, "method definition evaluation 40");
check(evalObj40.accessEval40 === 560 + 8, "getter definition evaluation 40");
evalObj40.accessEval40 = 560 + 14;
check(setterSinkEval40 === 560 + 14, "setter definition evaluation 40");

var orderStart41 = orderLog.length;
var shorthandValue41 = mark(573, 573 + 1);
var computedName41 = mark(573 + 2, "computedEval41");
var spreadSource41 = { spreadEval41: 573 + 3, overrideEval41: 573 + 4 };
var protoEval41 = { inheritedEval41: 573 + 5 };
var setterSinkEval41 = 0;
var evalObj41 = {
firstEval41: mark(573 + 6, 573 + 7),
shorthandValue41,
[computedName41]: mark(573 + 8, 573 + 9),
...markObject(573 + 10, spreadSource41),
overrideEval41: mark(573 + 11, 573 + 12),
__proto__: protoEval41,
methodEval41(extra) { return this.firstEval41 + this.overrideEval41 + extra; },
get accessEval41() { return this.firstEval41 + 1; },
set accessEval41(value) { setterSinkEval41 = value; },
};
check(orderLog[orderStart41] === 573, "order shorthand setup 41");
check(orderLog[orderStart41 + 1] === 573 + 2, "order computed name 41");
check(orderLog[orderStart41 + 2] === 573 + 6, "order first value 41");
check(orderLog[orderStart41 + 3] === 573 + 8, "order computed value 41");
check(orderLog[orderStart41 + 4] === 573 + 10, "order spread expression 41");
check(orderLog[orderStart41 + 5] === 573 + 11, "order override value 41");
check(evalObj41.firstEval41 === 573 + 7, "first data property 41");
check(evalObj41.shorthandValue41 === 573 + 1, "shorthand data property 41");
check(evalObj41[computedName41] === 573 + 9, "computed data property 41");
check(evalObj41.spreadEval41 === 573 + 3, "spread data property 41");
check(evalObj41.overrideEval41 === 573 + 12, "later property overrides spread 41");
check(Object.getPrototypeOf(evalObj41) === protoEval41, "literal proto setter 41");
check(evalObj41.inheritedEval41 === 573 + 5, "literal proto inherited 41");
check(evalObj41.methodEval41(2) === (573 + 7) + (573 + 12) + 2, "method definition evaluation 41");
check(evalObj41.accessEval41 === 573 + 8, "getter definition evaluation 41");
evalObj41.accessEval41 = 573 + 14;
check(setterSinkEval41 === 573 + 14, "setter definition evaluation 41");

var orderStart42 = orderLog.length;
var shorthandValue42 = mark(586, 586 + 1);
var computedName42 = mark(586 + 2, "computedEval42");
var spreadSource42 = { spreadEval42: 586 + 3, overrideEval42: 586 + 4 };
var protoEval42 = { inheritedEval42: 586 + 5 };
var setterSinkEval42 = 0;
var evalObj42 = {
firstEval42: mark(586 + 6, 586 + 7),
shorthandValue42,
[computedName42]: mark(586 + 8, 586 + 9),
...markObject(586 + 10, spreadSource42),
overrideEval42: mark(586 + 11, 586 + 12),
__proto__: protoEval42,
methodEval42(extra) { return this.firstEval42 + this.overrideEval42 + extra; },
get accessEval42() { return this.firstEval42 + 1; },
set accessEval42(value) { setterSinkEval42 = value; },
};
check(orderLog[orderStart42] === 586, "order shorthand setup 42");
check(orderLog[orderStart42 + 1] === 586 + 2, "order computed name 42");
check(orderLog[orderStart42 + 2] === 586 + 6, "order first value 42");
check(orderLog[orderStart42 + 3] === 586 + 8, "order computed value 42");
check(orderLog[orderStart42 + 4] === 586 + 10, "order spread expression 42");
check(orderLog[orderStart42 + 5] === 586 + 11, "order override value 42");
check(evalObj42.firstEval42 === 586 + 7, "first data property 42");
check(evalObj42.shorthandValue42 === 586 + 1, "shorthand data property 42");
check(evalObj42[computedName42] === 586 + 9, "computed data property 42");
check(evalObj42.spreadEval42 === 586 + 3, "spread data property 42");
check(evalObj42.overrideEval42 === 586 + 12, "later property overrides spread 42");
check(Object.getPrototypeOf(evalObj42) === protoEval42, "literal proto setter 42");
check(evalObj42.inheritedEval42 === 586 + 5, "literal proto inherited 42");
check(evalObj42.methodEval42(2) === (586 + 7) + (586 + 12) + 2, "method definition evaluation 42");
check(evalObj42.accessEval42 === 586 + 8, "getter definition evaluation 42");
evalObj42.accessEval42 = 586 + 14;
check(setterSinkEval42 === 586 + 14, "setter definition evaluation 42");

var orderStart43 = orderLog.length;
var shorthandValue43 = mark(599, 599 + 1);
var computedName43 = mark(599 + 2, "computedEval43");
var spreadSource43 = { spreadEval43: 599 + 3, overrideEval43: 599 + 4 };
var protoEval43 = { inheritedEval43: 599 + 5 };
var setterSinkEval43 = 0;
var evalObj43 = {
firstEval43: mark(599 + 6, 599 + 7),
shorthandValue43,
[computedName43]: mark(599 + 8, 599 + 9),
...markObject(599 + 10, spreadSource43),
overrideEval43: mark(599 + 11, 599 + 12),
__proto__: protoEval43,
methodEval43(extra) { return this.firstEval43 + this.overrideEval43 + extra; },
get accessEval43() { return this.firstEval43 + 1; },
set accessEval43(value) { setterSinkEval43 = value; },
};
check(orderLog[orderStart43] === 599, "order shorthand setup 43");
check(orderLog[orderStart43 + 1] === 599 + 2, "order computed name 43");
check(orderLog[orderStart43 + 2] === 599 + 6, "order first value 43");
check(orderLog[orderStart43 + 3] === 599 + 8, "order computed value 43");
check(orderLog[orderStart43 + 4] === 599 + 10, "order spread expression 43");
check(orderLog[orderStart43 + 5] === 599 + 11, "order override value 43");
check(evalObj43.firstEval43 === 599 + 7, "first data property 43");
check(evalObj43.shorthandValue43 === 599 + 1, "shorthand data property 43");
check(evalObj43[computedName43] === 599 + 9, "computed data property 43");
check(evalObj43.spreadEval43 === 599 + 3, "spread data property 43");
check(evalObj43.overrideEval43 === 599 + 12, "later property overrides spread 43");
check(Object.getPrototypeOf(evalObj43) === protoEval43, "literal proto setter 43");
check(evalObj43.inheritedEval43 === 599 + 5, "literal proto inherited 43");
check(evalObj43.methodEval43(2) === (599 + 7) + (599 + 12) + 2, "method definition evaluation 43");
check(evalObj43.accessEval43 === 599 + 8, "getter definition evaluation 43");
evalObj43.accessEval43 = 599 + 14;
check(setterSinkEval43 === 599 + 14, "setter definition evaluation 43");

var orderStart44 = orderLog.length;
var shorthandValue44 = mark(612, 612 + 1);
var computedName44 = mark(612 + 2, "computedEval44");
var spreadSource44 = { spreadEval44: 612 + 3, overrideEval44: 612 + 4 };
var protoEval44 = { inheritedEval44: 612 + 5 };
var setterSinkEval44 = 0;
var evalObj44 = {
firstEval44: mark(612 + 6, 612 + 7),
shorthandValue44,
[computedName44]: mark(612 + 8, 612 + 9),
...markObject(612 + 10, spreadSource44),
overrideEval44: mark(612 + 11, 612 + 12),
__proto__: protoEval44,
methodEval44(extra) { return this.firstEval44 + this.overrideEval44 + extra; },
get accessEval44() { return this.firstEval44 + 1; },
set accessEval44(value) { setterSinkEval44 = value; },
};
check(orderLog[orderStart44] === 612, "order shorthand setup 44");
check(orderLog[orderStart44 + 1] === 612 + 2, "order computed name 44");
check(orderLog[orderStart44 + 2] === 612 + 6, "order first value 44");
check(orderLog[orderStart44 + 3] === 612 + 8, "order computed value 44");
check(orderLog[orderStart44 + 4] === 612 + 10, "order spread expression 44");
check(orderLog[orderStart44 + 5] === 612 + 11, "order override value 44");
check(evalObj44.firstEval44 === 612 + 7, "first data property 44");
check(evalObj44.shorthandValue44 === 612 + 1, "shorthand data property 44");
check(evalObj44[computedName44] === 612 + 9, "computed data property 44");
check(evalObj44.spreadEval44 === 612 + 3, "spread data property 44");
check(evalObj44.overrideEval44 === 612 + 12, "later property overrides spread 44");
check(Object.getPrototypeOf(evalObj44) === protoEval44, "literal proto setter 44");
check(evalObj44.inheritedEval44 === 612 + 5, "literal proto inherited 44");
check(evalObj44.methodEval44(2) === (612 + 7) + (612 + 12) + 2, "method definition evaluation 44");
check(evalObj44.accessEval44 === 612 + 8, "getter definition evaluation 44");
evalObj44.accessEval44 = 612 + 14;
check(setterSinkEval44 === 612 + 14, "setter definition evaluation 44");

var orderStart45 = orderLog.length;
var shorthandValue45 = mark(625, 625 + 1);
var computedName45 = mark(625 + 2, "computedEval45");
var spreadSource45 = { spreadEval45: 625 + 3, overrideEval45: 625 + 4 };
var protoEval45 = { inheritedEval45: 625 + 5 };
var setterSinkEval45 = 0;
var evalObj45 = {
firstEval45: mark(625 + 6, 625 + 7),
shorthandValue45,
[computedName45]: mark(625 + 8, 625 + 9),
...markObject(625 + 10, spreadSource45),
overrideEval45: mark(625 + 11, 625 + 12),
__proto__: protoEval45,
methodEval45(extra) { return this.firstEval45 + this.overrideEval45 + extra; },
get accessEval45() { return this.firstEval45 + 1; },
set accessEval45(value) { setterSinkEval45 = value; },
};
check(orderLog[orderStart45] === 625, "order shorthand setup 45");
check(orderLog[orderStart45 + 1] === 625 + 2, "order computed name 45");
check(orderLog[orderStart45 + 2] === 625 + 6, "order first value 45");
check(orderLog[orderStart45 + 3] === 625 + 8, "order computed value 45");
check(orderLog[orderStart45 + 4] === 625 + 10, "order spread expression 45");
check(orderLog[orderStart45 + 5] === 625 + 11, "order override value 45");
check(evalObj45.firstEval45 === 625 + 7, "first data property 45");
check(evalObj45.shorthandValue45 === 625 + 1, "shorthand data property 45");
check(evalObj45[computedName45] === 625 + 9, "computed data property 45");
check(evalObj45.spreadEval45 === 625 + 3, "spread data property 45");
check(evalObj45.overrideEval45 === 625 + 12, "later property overrides spread 45");
check(Object.getPrototypeOf(evalObj45) === protoEval45, "literal proto setter 45");
check(evalObj45.inheritedEval45 === 625 + 5, "literal proto inherited 45");
check(evalObj45.methodEval45(2) === (625 + 7) + (625 + 12) + 2, "method definition evaluation 45");
check(evalObj45.accessEval45 === 625 + 8, "getter definition evaluation 45");
evalObj45.accessEval45 = 625 + 14;
check(setterSinkEval45 === 625 + 14, "setter definition evaluation 45");

var orderStart46 = orderLog.length;
var shorthandValue46 = mark(638, 638 + 1);
var computedName46 = mark(638 + 2, "computedEval46");
var spreadSource46 = { spreadEval46: 638 + 3, overrideEval46: 638 + 4 };
var protoEval46 = { inheritedEval46: 638 + 5 };
var setterSinkEval46 = 0;
var evalObj46 = {
firstEval46: mark(638 + 6, 638 + 7),
shorthandValue46,
[computedName46]: mark(638 + 8, 638 + 9),
...markObject(638 + 10, spreadSource46),
overrideEval46: mark(638 + 11, 638 + 12),
__proto__: protoEval46,
methodEval46(extra) { return this.firstEval46 + this.overrideEval46 + extra; },
get accessEval46() { return this.firstEval46 + 1; },
set accessEval46(value) { setterSinkEval46 = value; },
};
check(orderLog[orderStart46] === 638, "order shorthand setup 46");
check(orderLog[orderStart46 + 1] === 638 + 2, "order computed name 46");
check(orderLog[orderStart46 + 2] === 638 + 6, "order first value 46");
check(orderLog[orderStart46 + 3] === 638 + 8, "order computed value 46");
check(orderLog[orderStart46 + 4] === 638 + 10, "order spread expression 46");
check(orderLog[orderStart46 + 5] === 638 + 11, "order override value 46");
check(evalObj46.firstEval46 === 638 + 7, "first data property 46");
check(evalObj46.shorthandValue46 === 638 + 1, "shorthand data property 46");
check(evalObj46[computedName46] === 638 + 9, "computed data property 46");
check(evalObj46.spreadEval46 === 638 + 3, "spread data property 46");
check(evalObj46.overrideEval46 === 638 + 12, "later property overrides spread 46");
check(Object.getPrototypeOf(evalObj46) === protoEval46, "literal proto setter 46");
check(evalObj46.inheritedEval46 === 638 + 5, "literal proto inherited 46");
check(evalObj46.methodEval46(2) === (638 + 7) + (638 + 12) + 2, "method definition evaluation 46");
check(evalObj46.accessEval46 === 638 + 8, "getter definition evaluation 46");
evalObj46.accessEval46 = 638 + 14;
check(setterSinkEval46 === 638 + 14, "setter definition evaluation 46");

var orderStart47 = orderLog.length;
var shorthandValue47 = mark(651, 651 + 1);
var computedName47 = mark(651 + 2, "computedEval47");
var spreadSource47 = { spreadEval47: 651 + 3, overrideEval47: 651 + 4 };
var protoEval47 = { inheritedEval47: 651 + 5 };
var setterSinkEval47 = 0;
var evalObj47 = {
firstEval47: mark(651 + 6, 651 + 7),
shorthandValue47,
[computedName47]: mark(651 + 8, 651 + 9),
...markObject(651 + 10, spreadSource47),
overrideEval47: mark(651 + 11, 651 + 12),
__proto__: protoEval47,
methodEval47(extra) { return this.firstEval47 + this.overrideEval47 + extra; },
get accessEval47() { return this.firstEval47 + 1; },
set accessEval47(value) { setterSinkEval47 = value; },
};
check(orderLog[orderStart47] === 651, "order shorthand setup 47");
check(orderLog[orderStart47 + 1] === 651 + 2, "order computed name 47");
check(orderLog[orderStart47 + 2] === 651 + 6, "order first value 47");
check(orderLog[orderStart47 + 3] === 651 + 8, "order computed value 47");
check(orderLog[orderStart47 + 4] === 651 + 10, "order spread expression 47");
check(orderLog[orderStart47 + 5] === 651 + 11, "order override value 47");
check(evalObj47.firstEval47 === 651 + 7, "first data property 47");
check(evalObj47.shorthandValue47 === 651 + 1, "shorthand data property 47");
check(evalObj47[computedName47] === 651 + 9, "computed data property 47");
check(evalObj47.spreadEval47 === 651 + 3, "spread data property 47");
check(evalObj47.overrideEval47 === 651 + 12, "later property overrides spread 47");
check(Object.getPrototypeOf(evalObj47) === protoEval47, "literal proto setter 47");
check(evalObj47.inheritedEval47 === 651 + 5, "literal proto inherited 47");
check(evalObj47.methodEval47(2) === (651 + 7) + (651 + 12) + 2, "method definition evaluation 47");
check(evalObj47.accessEval47 === 651 + 8, "getter definition evaluation 47");
evalObj47.accessEval47 = 651 + 14;
check(setterSinkEval47 === 651 + 14, "setter definition evaluation 47");

var orderStart48 = orderLog.length;
var shorthandValue48 = mark(664, 664 + 1);
var computedName48 = mark(664 + 2, "computedEval48");
var spreadSource48 = { spreadEval48: 664 + 3, overrideEval48: 664 + 4 };
var protoEval48 = { inheritedEval48: 664 + 5 };
var setterSinkEval48 = 0;
var evalObj48 = {
firstEval48: mark(664 + 6, 664 + 7),
shorthandValue48,
[computedName48]: mark(664 + 8, 664 + 9),
...markObject(664 + 10, spreadSource48),
overrideEval48: mark(664 + 11, 664 + 12),
__proto__: protoEval48,
methodEval48(extra) { return this.firstEval48 + this.overrideEval48 + extra; },
get accessEval48() { return this.firstEval48 + 1; },
set accessEval48(value) { setterSinkEval48 = value; },
};
check(orderLog[orderStart48] === 664, "order shorthand setup 48");
check(orderLog[orderStart48 + 1] === 664 + 2, "order computed name 48");
check(orderLog[orderStart48 + 2] === 664 + 6, "order first value 48");
check(orderLog[orderStart48 + 3] === 664 + 8, "order computed value 48");
check(orderLog[orderStart48 + 4] === 664 + 10, "order spread expression 48");
check(orderLog[orderStart48 + 5] === 664 + 11, "order override value 48");
check(evalObj48.firstEval48 === 664 + 7, "first data property 48");
check(evalObj48.shorthandValue48 === 664 + 1, "shorthand data property 48");
check(evalObj48[computedName48] === 664 + 9, "computed data property 48");
check(evalObj48.spreadEval48 === 664 + 3, "spread data property 48");
check(evalObj48.overrideEval48 === 664 + 12, "later property overrides spread 48");
check(Object.getPrototypeOf(evalObj48) === protoEval48, "literal proto setter 48");
check(evalObj48.inheritedEval48 === 664 + 5, "literal proto inherited 48");
check(evalObj48.methodEval48(2) === (664 + 7) + (664 + 12) + 2, "method definition evaluation 48");
check(evalObj48.accessEval48 === 664 + 8, "getter definition evaluation 48");
evalObj48.accessEval48 = 664 + 14;
check(setterSinkEval48 === 664 + 14, "setter definition evaluation 48");

var orderStart49 = orderLog.length;
var shorthandValue49 = mark(677, 677 + 1);
var computedName49 = mark(677 + 2, "computedEval49");
var spreadSource49 = { spreadEval49: 677 + 3, overrideEval49: 677 + 4 };
var protoEval49 = { inheritedEval49: 677 + 5 };
var setterSinkEval49 = 0;
var evalObj49 = {
firstEval49: mark(677 + 6, 677 + 7),
shorthandValue49,
[computedName49]: mark(677 + 8, 677 + 9),
...markObject(677 + 10, spreadSource49),
overrideEval49: mark(677 + 11, 677 + 12),
__proto__: protoEval49,
methodEval49(extra) { return this.firstEval49 + this.overrideEval49 + extra; },
get accessEval49() { return this.firstEval49 + 1; },
set accessEval49(value) { setterSinkEval49 = value; },
};
check(orderLog[orderStart49] === 677, "order shorthand setup 49");
check(orderLog[orderStart49 + 1] === 677 + 2, "order computed name 49");
check(orderLog[orderStart49 + 2] === 677 + 6, "order first value 49");
check(orderLog[orderStart49 + 3] === 677 + 8, "order computed value 49");
check(orderLog[orderStart49 + 4] === 677 + 10, "order spread expression 49");
check(orderLog[orderStart49 + 5] === 677 + 11, "order override value 49");
check(evalObj49.firstEval49 === 677 + 7, "first data property 49");
check(evalObj49.shorthandValue49 === 677 + 1, "shorthand data property 49");
check(evalObj49[computedName49] === 677 + 9, "computed data property 49");
check(evalObj49.spreadEval49 === 677 + 3, "spread data property 49");
check(evalObj49.overrideEval49 === 677 + 12, "later property overrides spread 49");
check(Object.getPrototypeOf(evalObj49) === protoEval49, "literal proto setter 49");
check(evalObj49.inheritedEval49 === 677 + 5, "literal proto inherited 49");
check(evalObj49.methodEval49(2) === (677 + 7) + (677 + 12) + 2, "method definition evaluation 49");
check(evalObj49.accessEval49 === 677 + 8, "getter definition evaluation 49");
evalObj49.accessEval49 = 677 + 14;
check(setterSinkEval49 === 677 + 14, "setter definition evaluation 49");

var orderStart50 = orderLog.length;
var shorthandValue50 = mark(690, 690 + 1);
var computedName50 = mark(690 + 2, "computedEval50");
var spreadSource50 = { spreadEval50: 690 + 3, overrideEval50: 690 + 4 };
var protoEval50 = { inheritedEval50: 690 + 5 };
var setterSinkEval50 = 0;
var evalObj50 = {
firstEval50: mark(690 + 6, 690 + 7),
shorthandValue50,
[computedName50]: mark(690 + 8, 690 + 9),
...markObject(690 + 10, spreadSource50),
overrideEval50: mark(690 + 11, 690 + 12),
__proto__: protoEval50,
methodEval50(extra) { return this.firstEval50 + this.overrideEval50 + extra; },
get accessEval50() { return this.firstEval50 + 1; },
set accessEval50(value) { setterSinkEval50 = value; },
};
check(orderLog[orderStart50] === 690, "order shorthand setup 50");
check(orderLog[orderStart50 + 1] === 690 + 2, "order computed name 50");
check(orderLog[orderStart50 + 2] === 690 + 6, "order first value 50");
check(orderLog[orderStart50 + 3] === 690 + 8, "order computed value 50");
check(orderLog[orderStart50 + 4] === 690 + 10, "order spread expression 50");
check(orderLog[orderStart50 + 5] === 690 + 11, "order override value 50");
check(evalObj50.firstEval50 === 690 + 7, "first data property 50");
check(evalObj50.shorthandValue50 === 690 + 1, "shorthand data property 50");
check(evalObj50[computedName50] === 690 + 9, "computed data property 50");
check(evalObj50.spreadEval50 === 690 + 3, "spread data property 50");
check(evalObj50.overrideEval50 === 690 + 12, "later property overrides spread 50");
check(Object.getPrototypeOf(evalObj50) === protoEval50, "literal proto setter 50");
check(evalObj50.inheritedEval50 === 690 + 5, "literal proto inherited 50");
check(evalObj50.methodEval50(2) === (690 + 7) + (690 + 12) + 2, "method definition evaluation 50");
check(evalObj50.accessEval50 === 690 + 8, "getter definition evaluation 50");
evalObj50.accessEval50 = 690 + 14;
check(setterSinkEval50 === 690 + 14, "setter definition evaluation 50");

var orderStart51 = orderLog.length;
var shorthandValue51 = mark(703, 703 + 1);
var computedName51 = mark(703 + 2, "computedEval51");
var spreadSource51 = { spreadEval51: 703 + 3, overrideEval51: 703 + 4 };
var protoEval51 = { inheritedEval51: 703 + 5 };
var setterSinkEval51 = 0;
var evalObj51 = {
firstEval51: mark(703 + 6, 703 + 7),
shorthandValue51,
[computedName51]: mark(703 + 8, 703 + 9),
...markObject(703 + 10, spreadSource51),
overrideEval51: mark(703 + 11, 703 + 12),
__proto__: protoEval51,
methodEval51(extra) { return this.firstEval51 + this.overrideEval51 + extra; },
get accessEval51() { return this.firstEval51 + 1; },
set accessEval51(value) { setterSinkEval51 = value; },
};
check(orderLog[orderStart51] === 703, "order shorthand setup 51");
check(orderLog[orderStart51 + 1] === 703 + 2, "order computed name 51");
check(orderLog[orderStart51 + 2] === 703 + 6, "order first value 51");
check(orderLog[orderStart51 + 3] === 703 + 8, "order computed value 51");
check(orderLog[orderStart51 + 4] === 703 + 10, "order spread expression 51");
check(orderLog[orderStart51 + 5] === 703 + 11, "order override value 51");
check(evalObj51.firstEval51 === 703 + 7, "first data property 51");
check(evalObj51.shorthandValue51 === 703 + 1, "shorthand data property 51");
check(evalObj51[computedName51] === 703 + 9, "computed data property 51");
check(evalObj51.spreadEval51 === 703 + 3, "spread data property 51");
check(evalObj51.overrideEval51 === 703 + 12, "later property overrides spread 51");
check(Object.getPrototypeOf(evalObj51) === protoEval51, "literal proto setter 51");
check(evalObj51.inheritedEval51 === 703 + 5, "literal proto inherited 51");
check(evalObj51.methodEval51(2) === (703 + 7) + (703 + 12) + 2, "method definition evaluation 51");
check(evalObj51.accessEval51 === 703 + 8, "getter definition evaluation 51");
evalObj51.accessEval51 = 703 + 14;
check(setterSinkEval51 === 703 + 14, "setter definition evaluation 51");

var orderStart52 = orderLog.length;
var shorthandValue52 = mark(716, 716 + 1);
var computedName52 = mark(716 + 2, "computedEval52");
var spreadSource52 = { spreadEval52: 716 + 3, overrideEval52: 716 + 4 };
var protoEval52 = { inheritedEval52: 716 + 5 };
var setterSinkEval52 = 0;
var evalObj52 = {
firstEval52: mark(716 + 6, 716 + 7),
shorthandValue52,
[computedName52]: mark(716 + 8, 716 + 9),
...markObject(716 + 10, spreadSource52),
overrideEval52: mark(716 + 11, 716 + 12),
__proto__: protoEval52,
methodEval52(extra) { return this.firstEval52 + this.overrideEval52 + extra; },
get accessEval52() { return this.firstEval52 + 1; },
set accessEval52(value) { setterSinkEval52 = value; },
};
check(orderLog[orderStart52] === 716, "order shorthand setup 52");
check(orderLog[orderStart52 + 1] === 716 + 2, "order computed name 52");
check(orderLog[orderStart52 + 2] === 716 + 6, "order first value 52");
check(orderLog[orderStart52 + 3] === 716 + 8, "order computed value 52");
check(orderLog[orderStart52 + 4] === 716 + 10, "order spread expression 52");
check(orderLog[orderStart52 + 5] === 716 + 11, "order override value 52");
check(evalObj52.firstEval52 === 716 + 7, "first data property 52");
check(evalObj52.shorthandValue52 === 716 + 1, "shorthand data property 52");
check(evalObj52[computedName52] === 716 + 9, "computed data property 52");
check(evalObj52.spreadEval52 === 716 + 3, "spread data property 52");
check(evalObj52.overrideEval52 === 716 + 12, "later property overrides spread 52");
check(Object.getPrototypeOf(evalObj52) === protoEval52, "literal proto setter 52");
check(evalObj52.inheritedEval52 === 716 + 5, "literal proto inherited 52");
check(evalObj52.methodEval52(2) === (716 + 7) + (716 + 12) + 2, "method definition evaluation 52");
check(evalObj52.accessEval52 === 716 + 8, "getter definition evaluation 52");
evalObj52.accessEval52 = 716 + 14;
check(setterSinkEval52 === 716 + 14, "setter definition evaluation 52");

var orderStart53 = orderLog.length;
var shorthandValue53 = mark(729, 729 + 1);
var computedName53 = mark(729 + 2, "computedEval53");
var spreadSource53 = { spreadEval53: 729 + 3, overrideEval53: 729 + 4 };
var protoEval53 = { inheritedEval53: 729 + 5 };
var setterSinkEval53 = 0;
var evalObj53 = {
firstEval53: mark(729 + 6, 729 + 7),
shorthandValue53,
[computedName53]: mark(729 + 8, 729 + 9),
...markObject(729 + 10, spreadSource53),
overrideEval53: mark(729 + 11, 729 + 12),
__proto__: protoEval53,
methodEval53(extra) { return this.firstEval53 + this.overrideEval53 + extra; },
get accessEval53() { return this.firstEval53 + 1; },
set accessEval53(value) { setterSinkEval53 = value; },
};
check(orderLog[orderStart53] === 729, "order shorthand setup 53");
check(orderLog[orderStart53 + 1] === 729 + 2, "order computed name 53");
check(orderLog[orderStart53 + 2] === 729 + 6, "order first value 53");
check(orderLog[orderStart53 + 3] === 729 + 8, "order computed value 53");
check(orderLog[orderStart53 + 4] === 729 + 10, "order spread expression 53");
check(orderLog[orderStart53 + 5] === 729 + 11, "order override value 53");
check(evalObj53.firstEval53 === 729 + 7, "first data property 53");
check(evalObj53.shorthandValue53 === 729 + 1, "shorthand data property 53");
check(evalObj53[computedName53] === 729 + 9, "computed data property 53");
check(evalObj53.spreadEval53 === 729 + 3, "spread data property 53");
check(evalObj53.overrideEval53 === 729 + 12, "later property overrides spread 53");
check(Object.getPrototypeOf(evalObj53) === protoEval53, "literal proto setter 53");
check(evalObj53.inheritedEval53 === 729 + 5, "literal proto inherited 53");
check(evalObj53.methodEval53(2) === (729 + 7) + (729 + 12) + 2, "method definition evaluation 53");
check(evalObj53.accessEval53 === 729 + 8, "getter definition evaluation 53");
evalObj53.accessEval53 = 729 + 14;
check(setterSinkEval53 === 729 + 14, "setter definition evaluation 53");

var orderStart54 = orderLog.length;
var shorthandValue54 = mark(742, 742 + 1);
var computedName54 = mark(742 + 2, "computedEval54");
var spreadSource54 = { spreadEval54: 742 + 3, overrideEval54: 742 + 4 };
var protoEval54 = { inheritedEval54: 742 + 5 };
var setterSinkEval54 = 0;
var evalObj54 = {
firstEval54: mark(742 + 6, 742 + 7),
shorthandValue54,
[computedName54]: mark(742 + 8, 742 + 9),
...markObject(742 + 10, spreadSource54),
overrideEval54: mark(742 + 11, 742 + 12),
__proto__: protoEval54,
methodEval54(extra) { return this.firstEval54 + this.overrideEval54 + extra; },
get accessEval54() { return this.firstEval54 + 1; },
set accessEval54(value) { setterSinkEval54 = value; },
};
check(orderLog[orderStart54] === 742, "order shorthand setup 54");
check(orderLog[orderStart54 + 1] === 742 + 2, "order computed name 54");
check(orderLog[orderStart54 + 2] === 742 + 6, "order first value 54");
check(orderLog[orderStart54 + 3] === 742 + 8, "order computed value 54");
check(orderLog[orderStart54 + 4] === 742 + 10, "order spread expression 54");
check(orderLog[orderStart54 + 5] === 742 + 11, "order override value 54");
check(evalObj54.firstEval54 === 742 + 7, "first data property 54");
check(evalObj54.shorthandValue54 === 742 + 1, "shorthand data property 54");
check(evalObj54[computedName54] === 742 + 9, "computed data property 54");
check(evalObj54.spreadEval54 === 742 + 3, "spread data property 54");
check(evalObj54.overrideEval54 === 742 + 12, "later property overrides spread 54");
check(Object.getPrototypeOf(evalObj54) === protoEval54, "literal proto setter 54");
check(evalObj54.inheritedEval54 === 742 + 5, "literal proto inherited 54");
check(evalObj54.methodEval54(2) === (742 + 7) + (742 + 12) + 2, "method definition evaluation 54");
check(evalObj54.accessEval54 === 742 + 8, "getter definition evaluation 54");
evalObj54.accessEval54 = 742 + 14;
check(setterSinkEval54 === 742 + 14, "setter definition evaluation 54");

var orderStart55 = orderLog.length;
var shorthandValue55 = mark(755, 755 + 1);
var computedName55 = mark(755 + 2, "computedEval55");
var spreadSource55 = { spreadEval55: 755 + 3, overrideEval55: 755 + 4 };
var protoEval55 = { inheritedEval55: 755 + 5 };
var setterSinkEval55 = 0;
var evalObj55 = {
firstEval55: mark(755 + 6, 755 + 7),
shorthandValue55,
[computedName55]: mark(755 + 8, 755 + 9),
...markObject(755 + 10, spreadSource55),
overrideEval55: mark(755 + 11, 755 + 12),
__proto__: protoEval55,
methodEval55(extra) { return this.firstEval55 + this.overrideEval55 + extra; },
get accessEval55() { return this.firstEval55 + 1; },
set accessEval55(value) { setterSinkEval55 = value; },
};
check(orderLog[orderStart55] === 755, "order shorthand setup 55");
check(orderLog[orderStart55 + 1] === 755 + 2, "order computed name 55");
check(orderLog[orderStart55 + 2] === 755 + 6, "order first value 55");
check(orderLog[orderStart55 + 3] === 755 + 8, "order computed value 55");
check(orderLog[orderStart55 + 4] === 755 + 10, "order spread expression 55");
check(orderLog[orderStart55 + 5] === 755 + 11, "order override value 55");
check(evalObj55.firstEval55 === 755 + 7, "first data property 55");
check(evalObj55.shorthandValue55 === 755 + 1, "shorthand data property 55");
check(evalObj55[computedName55] === 755 + 9, "computed data property 55");
check(evalObj55.spreadEval55 === 755 + 3, "spread data property 55");
check(evalObj55.overrideEval55 === 755 + 12, "later property overrides spread 55");
check(Object.getPrototypeOf(evalObj55) === protoEval55, "literal proto setter 55");
check(evalObj55.inheritedEval55 === 755 + 5, "literal proto inherited 55");
check(evalObj55.methodEval55(2) === (755 + 7) + (755 + 12) + 2, "method definition evaluation 55");
check(evalObj55.accessEval55 === 755 + 8, "getter definition evaluation 55");
evalObj55.accessEval55 = 755 + 14;
check(setterSinkEval55 === 755 + 14, "setter definition evaluation 55");

var orderStart56 = orderLog.length;
var shorthandValue56 = mark(768, 768 + 1);
var computedName56 = mark(768 + 2, "computedEval56");
var spreadSource56 = { spreadEval56: 768 + 3, overrideEval56: 768 + 4 };
var protoEval56 = { inheritedEval56: 768 + 5 };
var setterSinkEval56 = 0;
var evalObj56 = {
firstEval56: mark(768 + 6, 768 + 7),
shorthandValue56,
[computedName56]: mark(768 + 8, 768 + 9),
...markObject(768 + 10, spreadSource56),
overrideEval56: mark(768 + 11, 768 + 12),
__proto__: protoEval56,
methodEval56(extra) { return this.firstEval56 + this.overrideEval56 + extra; },
get accessEval56() { return this.firstEval56 + 1; },
set accessEval56(value) { setterSinkEval56 = value; },
};
check(orderLog[orderStart56] === 768, "order shorthand setup 56");
check(orderLog[orderStart56 + 1] === 768 + 2, "order computed name 56");
check(orderLog[orderStart56 + 2] === 768 + 6, "order first value 56");
check(orderLog[orderStart56 + 3] === 768 + 8, "order computed value 56");
check(orderLog[orderStart56 + 4] === 768 + 10, "order spread expression 56");
check(orderLog[orderStart56 + 5] === 768 + 11, "order override value 56");
check(evalObj56.firstEval56 === 768 + 7, "first data property 56");
check(evalObj56.shorthandValue56 === 768 + 1, "shorthand data property 56");
check(evalObj56[computedName56] === 768 + 9, "computed data property 56");
check(evalObj56.spreadEval56 === 768 + 3, "spread data property 56");
check(evalObj56.overrideEval56 === 768 + 12, "later property overrides spread 56");
check(Object.getPrototypeOf(evalObj56) === protoEval56, "literal proto setter 56");
check(evalObj56.inheritedEval56 === 768 + 5, "literal proto inherited 56");
check(evalObj56.methodEval56(2) === (768 + 7) + (768 + 12) + 2, "method definition evaluation 56");
check(evalObj56.accessEval56 === 768 + 8, "getter definition evaluation 56");
evalObj56.accessEval56 = 768 + 14;
check(setterSinkEval56 === 768 + 14, "setter definition evaluation 56");

var orderStart57 = orderLog.length;
var shorthandValue57 = mark(781, 781 + 1);
var computedName57 = mark(781 + 2, "computedEval57");
var spreadSource57 = { spreadEval57: 781 + 3, overrideEval57: 781 + 4 };
var protoEval57 = { inheritedEval57: 781 + 5 };
var setterSinkEval57 = 0;
var evalObj57 = {
firstEval57: mark(781 + 6, 781 + 7),
shorthandValue57,
[computedName57]: mark(781 + 8, 781 + 9),
...markObject(781 + 10, spreadSource57),
overrideEval57: mark(781 + 11, 781 + 12),
__proto__: protoEval57,
methodEval57(extra) { return this.firstEval57 + this.overrideEval57 + extra; },
get accessEval57() { return this.firstEval57 + 1; },
set accessEval57(value) { setterSinkEval57 = value; },
};
check(orderLog[orderStart57] === 781, "order shorthand setup 57");
check(orderLog[orderStart57 + 1] === 781 + 2, "order computed name 57");
check(orderLog[orderStart57 + 2] === 781 + 6, "order first value 57");
check(orderLog[orderStart57 + 3] === 781 + 8, "order computed value 57");
check(orderLog[orderStart57 + 4] === 781 + 10, "order spread expression 57");
check(orderLog[orderStart57 + 5] === 781 + 11, "order override value 57");
check(evalObj57.firstEval57 === 781 + 7, "first data property 57");
check(evalObj57.shorthandValue57 === 781 + 1, "shorthand data property 57");
check(evalObj57[computedName57] === 781 + 9, "computed data property 57");
check(evalObj57.spreadEval57 === 781 + 3, "spread data property 57");
check(evalObj57.overrideEval57 === 781 + 12, "later property overrides spread 57");
check(Object.getPrototypeOf(evalObj57) === protoEval57, "literal proto setter 57");
check(evalObj57.inheritedEval57 === 781 + 5, "literal proto inherited 57");
check(evalObj57.methodEval57(2) === (781 + 7) + (781 + 12) + 2, "method definition evaluation 57");
check(evalObj57.accessEval57 === 781 + 8, "getter definition evaluation 57");
evalObj57.accessEval57 = 781 + 14;
check(setterSinkEval57 === 781 + 14, "setter definition evaluation 57");

var orderStart58 = orderLog.length;
var shorthandValue58 = mark(794, 794 + 1);
var computedName58 = mark(794 + 2, "computedEval58");
var spreadSource58 = { spreadEval58: 794 + 3, overrideEval58: 794 + 4 };
var protoEval58 = { inheritedEval58: 794 + 5 };
var setterSinkEval58 = 0;
var evalObj58 = {
firstEval58: mark(794 + 6, 794 + 7),
shorthandValue58,
[computedName58]: mark(794 + 8, 794 + 9),
...markObject(794 + 10, spreadSource58),
overrideEval58: mark(794 + 11, 794 + 12),
__proto__: protoEval58,
methodEval58(extra) { return this.firstEval58 + this.overrideEval58 + extra; },
get accessEval58() { return this.firstEval58 + 1; },
set accessEval58(value) { setterSinkEval58 = value; },
};
check(orderLog[orderStart58] === 794, "order shorthand setup 58");
check(orderLog[orderStart58 + 1] === 794 + 2, "order computed name 58");
check(orderLog[orderStart58 + 2] === 794 + 6, "order first value 58");
check(orderLog[orderStart58 + 3] === 794 + 8, "order computed value 58");
check(orderLog[orderStart58 + 4] === 794 + 10, "order spread expression 58");
check(orderLog[orderStart58 + 5] === 794 + 11, "order override value 58");
check(evalObj58.firstEval58 === 794 + 7, "first data property 58");
check(evalObj58.shorthandValue58 === 794 + 1, "shorthand data property 58");
check(evalObj58[computedName58] === 794 + 9, "computed data property 58");
check(evalObj58.spreadEval58 === 794 + 3, "spread data property 58");
check(evalObj58.overrideEval58 === 794 + 12, "later property overrides spread 58");
check(Object.getPrototypeOf(evalObj58) === protoEval58, "literal proto setter 58");
check(evalObj58.inheritedEval58 === 794 + 5, "literal proto inherited 58");
check(evalObj58.methodEval58(2) === (794 + 7) + (794 + 12) + 2, "method definition evaluation 58");
check(evalObj58.accessEval58 === 794 + 8, "getter definition evaluation 58");
evalObj58.accessEval58 = 794 + 14;
check(setterSinkEval58 === 794 + 14, "setter definition evaluation 58");

var orderStart59 = orderLog.length;
var shorthandValue59 = mark(807, 807 + 1);
var computedName59 = mark(807 + 2, "computedEval59");
var spreadSource59 = { spreadEval59: 807 + 3, overrideEval59: 807 + 4 };
var protoEval59 = { inheritedEval59: 807 + 5 };
var setterSinkEval59 = 0;
var evalObj59 = {
firstEval59: mark(807 + 6, 807 + 7),
shorthandValue59,
[computedName59]: mark(807 + 8, 807 + 9),
...markObject(807 + 10, spreadSource59),
overrideEval59: mark(807 + 11, 807 + 12),
__proto__: protoEval59,
methodEval59(extra) { return this.firstEval59 + this.overrideEval59 + extra; },
get accessEval59() { return this.firstEval59 + 1; },
set accessEval59(value) { setterSinkEval59 = value; },
};
check(orderLog[orderStart59] === 807, "order shorthand setup 59");
check(orderLog[orderStart59 + 1] === 807 + 2, "order computed name 59");
check(orderLog[orderStart59 + 2] === 807 + 6, "order first value 59");
check(orderLog[orderStart59 + 3] === 807 + 8, "order computed value 59");
check(orderLog[orderStart59 + 4] === 807 + 10, "order spread expression 59");
check(orderLog[orderStart59 + 5] === 807 + 11, "order override value 59");
check(evalObj59.firstEval59 === 807 + 7, "first data property 59");
check(evalObj59.shorthandValue59 === 807 + 1, "shorthand data property 59");
check(evalObj59[computedName59] === 807 + 9, "computed data property 59");
check(evalObj59.spreadEval59 === 807 + 3, "spread data property 59");
check(evalObj59.overrideEval59 === 807 + 12, "later property overrides spread 59");
check(Object.getPrototypeOf(evalObj59) === protoEval59, "literal proto setter 59");
check(evalObj59.inheritedEval59 === 807 + 5, "literal proto inherited 59");
check(evalObj59.methodEval59(2) === (807 + 7) + (807 + 12) + 2, "method definition evaluation 59");
check(evalObj59.accessEval59 === 807 + 8, "getter definition evaluation 59");
evalObj59.accessEval59 = 807 + 14;
check(setterSinkEval59 === 807 + 14, "setter definition evaluation 59");

var orderStart60 = orderLog.length;
var shorthandValue60 = mark(820, 820 + 1);
var computedName60 = mark(820 + 2, "computedEval60");
var spreadSource60 = { spreadEval60: 820 + 3, overrideEval60: 820 + 4 };
var protoEval60 = { inheritedEval60: 820 + 5 };
var setterSinkEval60 = 0;
var evalObj60 = {
firstEval60: mark(820 + 6, 820 + 7),
shorthandValue60,
[computedName60]: mark(820 + 8, 820 + 9),
...markObject(820 + 10, spreadSource60),
overrideEval60: mark(820 + 11, 820 + 12),
__proto__: protoEval60,
methodEval60(extra) { return this.firstEval60 + this.overrideEval60 + extra; },
get accessEval60() { return this.firstEval60 + 1; },
set accessEval60(value) { setterSinkEval60 = value; },
};
check(orderLog[orderStart60] === 820, "order shorthand setup 60");
check(orderLog[orderStart60 + 1] === 820 + 2, "order computed name 60");
check(orderLog[orderStart60 + 2] === 820 + 6, "order first value 60");
check(orderLog[orderStart60 + 3] === 820 + 8, "order computed value 60");
check(orderLog[orderStart60 + 4] === 820 + 10, "order spread expression 60");
check(orderLog[orderStart60 + 5] === 820 + 11, "order override value 60");
check(evalObj60.firstEval60 === 820 + 7, "first data property 60");
check(evalObj60.shorthandValue60 === 820 + 1, "shorthand data property 60");
check(evalObj60[computedName60] === 820 + 9, "computed data property 60");
check(evalObj60.spreadEval60 === 820 + 3, "spread data property 60");
check(evalObj60.overrideEval60 === 820 + 12, "later property overrides spread 60");
check(Object.getPrototypeOf(evalObj60) === protoEval60, "literal proto setter 60");
check(evalObj60.inheritedEval60 === 820 + 5, "literal proto inherited 60");
check(evalObj60.methodEval60(2) === (820 + 7) + (820 + 12) + 2, "method definition evaluation 60");
check(evalObj60.accessEval60 === 820 + 8, "getter definition evaluation 60");
evalObj60.accessEval60 = 820 + 14;
check(setterSinkEval60 === 820 + 14, "setter definition evaluation 60");

var orderStart61 = orderLog.length;
var shorthandValue61 = mark(833, 833 + 1);
var computedName61 = mark(833 + 2, "computedEval61");
var spreadSource61 = { spreadEval61: 833 + 3, overrideEval61: 833 + 4 };
var protoEval61 = { inheritedEval61: 833 + 5 };
var setterSinkEval61 = 0;
var evalObj61 = {
firstEval61: mark(833 + 6, 833 + 7),
shorthandValue61,
[computedName61]: mark(833 + 8, 833 + 9),
...markObject(833 + 10, spreadSource61),
overrideEval61: mark(833 + 11, 833 + 12),
__proto__: protoEval61,
methodEval61(extra) { return this.firstEval61 + this.overrideEval61 + extra; },
get accessEval61() { return this.firstEval61 + 1; },
set accessEval61(value) { setterSinkEval61 = value; },
};
check(orderLog[orderStart61] === 833, "order shorthand setup 61");
check(orderLog[orderStart61 + 1] === 833 + 2, "order computed name 61");
check(orderLog[orderStart61 + 2] === 833 + 6, "order first value 61");
check(orderLog[orderStart61 + 3] === 833 + 8, "order computed value 61");
check(orderLog[orderStart61 + 4] === 833 + 10, "order spread expression 61");
check(orderLog[orderStart61 + 5] === 833 + 11, "order override value 61");
check(evalObj61.firstEval61 === 833 + 7, "first data property 61");
check(evalObj61.shorthandValue61 === 833 + 1, "shorthand data property 61");
check(evalObj61[computedName61] === 833 + 9, "computed data property 61");
check(evalObj61.spreadEval61 === 833 + 3, "spread data property 61");
check(evalObj61.overrideEval61 === 833 + 12, "later property overrides spread 61");
check(Object.getPrototypeOf(evalObj61) === protoEval61, "literal proto setter 61");
check(evalObj61.inheritedEval61 === 833 + 5, "literal proto inherited 61");
check(evalObj61.methodEval61(2) === (833 + 7) + (833 + 12) + 2, "method definition evaluation 61");
check(evalObj61.accessEval61 === 833 + 8, "getter definition evaluation 61");
evalObj61.accessEval61 = 833 + 14;
check(setterSinkEval61 === 833 + 14, "setter definition evaluation 61");

var orderStart62 = orderLog.length;
var shorthandValue62 = mark(846, 846 + 1);
var computedName62 = mark(846 + 2, "computedEval62");
var spreadSource62 = { spreadEval62: 846 + 3, overrideEval62: 846 + 4 };
var protoEval62 = { inheritedEval62: 846 + 5 };
var setterSinkEval62 = 0;
var evalObj62 = {
firstEval62: mark(846 + 6, 846 + 7),
shorthandValue62,
[computedName62]: mark(846 + 8, 846 + 9),
...markObject(846 + 10, spreadSource62),
overrideEval62: mark(846 + 11, 846 + 12),
__proto__: protoEval62,
methodEval62(extra) { return this.firstEval62 + this.overrideEval62 + extra; },
get accessEval62() { return this.firstEval62 + 1; },
set accessEval62(value) { setterSinkEval62 = value; },
};
check(orderLog[orderStart62] === 846, "order shorthand setup 62");
check(orderLog[orderStart62 + 1] === 846 + 2, "order computed name 62");
check(orderLog[orderStart62 + 2] === 846 + 6, "order first value 62");
check(orderLog[orderStart62 + 3] === 846 + 8, "order computed value 62");
check(orderLog[orderStart62 + 4] === 846 + 10, "order spread expression 62");
check(orderLog[orderStart62 + 5] === 846 + 11, "order override value 62");
check(evalObj62.firstEval62 === 846 + 7, "first data property 62");
check(evalObj62.shorthandValue62 === 846 + 1, "shorthand data property 62");
check(evalObj62[computedName62] === 846 + 9, "computed data property 62");
check(evalObj62.spreadEval62 === 846 + 3, "spread data property 62");
check(evalObj62.overrideEval62 === 846 + 12, "later property overrides spread 62");
check(Object.getPrototypeOf(evalObj62) === protoEval62, "literal proto setter 62");
check(evalObj62.inheritedEval62 === 846 + 5, "literal proto inherited 62");
check(evalObj62.methodEval62(2) === (846 + 7) + (846 + 12) + 2, "method definition evaluation 62");
check(evalObj62.accessEval62 === 846 + 8, "getter definition evaluation 62");
evalObj62.accessEval62 = 846 + 14;
check(setterSinkEval62 === 846 + 14, "setter definition evaluation 62");

var orderStart63 = orderLog.length;
var shorthandValue63 = mark(859, 859 + 1);
var computedName63 = mark(859 + 2, "computedEval63");
var spreadSource63 = { spreadEval63: 859 + 3, overrideEval63: 859 + 4 };
var protoEval63 = { inheritedEval63: 859 + 5 };
var setterSinkEval63 = 0;
var evalObj63 = {
firstEval63: mark(859 + 6, 859 + 7),
shorthandValue63,
[computedName63]: mark(859 + 8, 859 + 9),
...markObject(859 + 10, spreadSource63),
overrideEval63: mark(859 + 11, 859 + 12),
__proto__: protoEval63,
methodEval63(extra) { return this.firstEval63 + this.overrideEval63 + extra; },
get accessEval63() { return this.firstEval63 + 1; },
set accessEval63(value) { setterSinkEval63 = value; },
};
check(orderLog[orderStart63] === 859, "order shorthand setup 63");
check(orderLog[orderStart63 + 1] === 859 + 2, "order computed name 63");
check(orderLog[orderStart63 + 2] === 859 + 6, "order first value 63");
check(orderLog[orderStart63 + 3] === 859 + 8, "order computed value 63");
check(orderLog[orderStart63 + 4] === 859 + 10, "order spread expression 63");
check(orderLog[orderStart63 + 5] === 859 + 11, "order override value 63");
check(evalObj63.firstEval63 === 859 + 7, "first data property 63");
check(evalObj63.shorthandValue63 === 859 + 1, "shorthand data property 63");
check(evalObj63[computedName63] === 859 + 9, "computed data property 63");
check(evalObj63.spreadEval63 === 859 + 3, "spread data property 63");
check(evalObj63.overrideEval63 === 859 + 12, "later property overrides spread 63");
check(Object.getPrototypeOf(evalObj63) === protoEval63, "literal proto setter 63");
check(evalObj63.inheritedEval63 === 859 + 5, "literal proto inherited 63");
check(evalObj63.methodEval63(2) === (859 + 7) + (859 + 12) + 2, "method definition evaluation 63");
check(evalObj63.accessEval63 === 859 + 8, "getter definition evaluation 63");
evalObj63.accessEval63 = 859 + 14;
check(setterSinkEval63 === 859 + 14, "setter definition evaluation 63");

var orderStart64 = orderLog.length;
var shorthandValue64 = mark(872, 872 + 1);
var computedName64 = mark(872 + 2, "computedEval64");
var spreadSource64 = { spreadEval64: 872 + 3, overrideEval64: 872 + 4 };
var protoEval64 = { inheritedEval64: 872 + 5 };
var setterSinkEval64 = 0;
var evalObj64 = {
firstEval64: mark(872 + 6, 872 + 7),
shorthandValue64,
[computedName64]: mark(872 + 8, 872 + 9),
...markObject(872 + 10, spreadSource64),
overrideEval64: mark(872 + 11, 872 + 12),
__proto__: protoEval64,
methodEval64(extra) { return this.firstEval64 + this.overrideEval64 + extra; },
get accessEval64() { return this.firstEval64 + 1; },
set accessEval64(value) { setterSinkEval64 = value; },
};
check(orderLog[orderStart64] === 872, "order shorthand setup 64");
check(orderLog[orderStart64 + 1] === 872 + 2, "order computed name 64");
check(orderLog[orderStart64 + 2] === 872 + 6, "order first value 64");
check(orderLog[orderStart64 + 3] === 872 + 8, "order computed value 64");
check(orderLog[orderStart64 + 4] === 872 + 10, "order spread expression 64");
check(orderLog[orderStart64 + 5] === 872 + 11, "order override value 64");
check(evalObj64.firstEval64 === 872 + 7, "first data property 64");
check(evalObj64.shorthandValue64 === 872 + 1, "shorthand data property 64");
check(evalObj64[computedName64] === 872 + 9, "computed data property 64");
check(evalObj64.spreadEval64 === 872 + 3, "spread data property 64");
check(evalObj64.overrideEval64 === 872 + 12, "later property overrides spread 64");
check(Object.getPrototypeOf(evalObj64) === protoEval64, "literal proto setter 64");
check(evalObj64.inheritedEval64 === 872 + 5, "literal proto inherited 64");
check(evalObj64.methodEval64(2) === (872 + 7) + (872 + 12) + 2, "method definition evaluation 64");
check(evalObj64.accessEval64 === 872 + 8, "getter definition evaluation 64");
evalObj64.accessEval64 = 872 + 14;
check(setterSinkEval64 === 872 + 14, "setter definition evaluation 64");

var orderStart65 = orderLog.length;
var shorthandValue65 = mark(885, 885 + 1);
var computedName65 = mark(885 + 2, "computedEval65");
var spreadSource65 = { spreadEval65: 885 + 3, overrideEval65: 885 + 4 };
var protoEval65 = { inheritedEval65: 885 + 5 };
var setterSinkEval65 = 0;
var evalObj65 = {
firstEval65: mark(885 + 6, 885 + 7),
shorthandValue65,
[computedName65]: mark(885 + 8, 885 + 9),
...markObject(885 + 10, spreadSource65),
overrideEval65: mark(885 + 11, 885 + 12),
__proto__: protoEval65,
methodEval65(extra) { return this.firstEval65 + this.overrideEval65 + extra; },
get accessEval65() { return this.firstEval65 + 1; },
set accessEval65(value) { setterSinkEval65 = value; },
};
check(orderLog[orderStart65] === 885, "order shorthand setup 65");
check(orderLog[orderStart65 + 1] === 885 + 2, "order computed name 65");
check(orderLog[orderStart65 + 2] === 885 + 6, "order first value 65");
check(orderLog[orderStart65 + 3] === 885 + 8, "order computed value 65");
check(orderLog[orderStart65 + 4] === 885 + 10, "order spread expression 65");
check(orderLog[orderStart65 + 5] === 885 + 11, "order override value 65");
check(evalObj65.firstEval65 === 885 + 7, "first data property 65");
check(evalObj65.shorthandValue65 === 885 + 1, "shorthand data property 65");
check(evalObj65[computedName65] === 885 + 9, "computed data property 65");
check(evalObj65.spreadEval65 === 885 + 3, "spread data property 65");
check(evalObj65.overrideEval65 === 885 + 12, "later property overrides spread 65");
check(Object.getPrototypeOf(evalObj65) === protoEval65, "literal proto setter 65");
check(evalObj65.inheritedEval65 === 885 + 5, "literal proto inherited 65");
check(evalObj65.methodEval65(2) === (885 + 7) + (885 + 12) + 2, "method definition evaluation 65");
check(evalObj65.accessEval65 === 885 + 8, "getter definition evaluation 65");
evalObj65.accessEval65 = 885 + 14;
check(setterSinkEval65 === 885 + 14, "setter definition evaluation 65");

var orderStart66 = orderLog.length;
var shorthandValue66 = mark(898, 898 + 1);
var computedName66 = mark(898 + 2, "computedEval66");
var spreadSource66 = { spreadEval66: 898 + 3, overrideEval66: 898 + 4 };
var protoEval66 = { inheritedEval66: 898 + 5 };
var setterSinkEval66 = 0;
var evalObj66 = {
firstEval66: mark(898 + 6, 898 + 7),
shorthandValue66,
[computedName66]: mark(898 + 8, 898 + 9),
...markObject(898 + 10, spreadSource66),
overrideEval66: mark(898 + 11, 898 + 12),
__proto__: protoEval66,
methodEval66(extra) { return this.firstEval66 + this.overrideEval66 + extra; },
get accessEval66() { return this.firstEval66 + 1; },
set accessEval66(value) { setterSinkEval66 = value; },
};
check(orderLog[orderStart66] === 898, "order shorthand setup 66");
check(orderLog[orderStart66 + 1] === 898 + 2, "order computed name 66");
check(orderLog[orderStart66 + 2] === 898 + 6, "order first value 66");
check(orderLog[orderStart66 + 3] === 898 + 8, "order computed value 66");
check(orderLog[orderStart66 + 4] === 898 + 10, "order spread expression 66");
check(orderLog[orderStart66 + 5] === 898 + 11, "order override value 66");
check(evalObj66.firstEval66 === 898 + 7, "first data property 66");
check(evalObj66.shorthandValue66 === 898 + 1, "shorthand data property 66");
check(evalObj66[computedName66] === 898 + 9, "computed data property 66");
check(evalObj66.spreadEval66 === 898 + 3, "spread data property 66");
check(evalObj66.overrideEval66 === 898 + 12, "later property overrides spread 66");
check(Object.getPrototypeOf(evalObj66) === protoEval66, "literal proto setter 66");
check(evalObj66.inheritedEval66 === 898 + 5, "literal proto inherited 66");
check(evalObj66.methodEval66(2) === (898 + 7) + (898 + 12) + 2, "method definition evaluation 66");
check(evalObj66.accessEval66 === 898 + 8, "getter definition evaluation 66");
evalObj66.accessEval66 = 898 + 14;
check(setterSinkEval66 === 898 + 14, "setter definition evaluation 66");

var orderStart67 = orderLog.length;
var shorthandValue67 = mark(911, 911 + 1);
var computedName67 = mark(911 + 2, "computedEval67");
var spreadSource67 = { spreadEval67: 911 + 3, overrideEval67: 911 + 4 };
var protoEval67 = { inheritedEval67: 911 + 5 };
var setterSinkEval67 = 0;
var evalObj67 = {
firstEval67: mark(911 + 6, 911 + 7),
shorthandValue67,
[computedName67]: mark(911 + 8, 911 + 9),
...markObject(911 + 10, spreadSource67),
overrideEval67: mark(911 + 11, 911 + 12),
__proto__: protoEval67,
methodEval67(extra) { return this.firstEval67 + this.overrideEval67 + extra; },
get accessEval67() { return this.firstEval67 + 1; },
set accessEval67(value) { setterSinkEval67 = value; },
};
check(orderLog[orderStart67] === 911, "order shorthand setup 67");
check(orderLog[orderStart67 + 1] === 911 + 2, "order computed name 67");
check(orderLog[orderStart67 + 2] === 911 + 6, "order first value 67");
check(orderLog[orderStart67 + 3] === 911 + 8, "order computed value 67");
check(orderLog[orderStart67 + 4] === 911 + 10, "order spread expression 67");
check(orderLog[orderStart67 + 5] === 911 + 11, "order override value 67");
check(evalObj67.firstEval67 === 911 + 7, "first data property 67");
check(evalObj67.shorthandValue67 === 911 + 1, "shorthand data property 67");
check(evalObj67[computedName67] === 911 + 9, "computed data property 67");
check(evalObj67.spreadEval67 === 911 + 3, "spread data property 67");
check(evalObj67.overrideEval67 === 911 + 12, "later property overrides spread 67");
check(Object.getPrototypeOf(evalObj67) === protoEval67, "literal proto setter 67");
check(evalObj67.inheritedEval67 === 911 + 5, "literal proto inherited 67");
check(evalObj67.methodEval67(2) === (911 + 7) + (911 + 12) + 2, "method definition evaluation 67");
check(evalObj67.accessEval67 === 911 + 8, "getter definition evaluation 67");
evalObj67.accessEval67 = 911 + 14;
check(setterSinkEval67 === 911 + 14, "setter definition evaluation 67");

var orderStart68 = orderLog.length;
var shorthandValue68 = mark(924, 924 + 1);
var computedName68 = mark(924 + 2, "computedEval68");
var spreadSource68 = { spreadEval68: 924 + 3, overrideEval68: 924 + 4 };
var protoEval68 = { inheritedEval68: 924 + 5 };
var setterSinkEval68 = 0;
var evalObj68 = {
firstEval68: mark(924 + 6, 924 + 7),
shorthandValue68,
[computedName68]: mark(924 + 8, 924 + 9),
...markObject(924 + 10, spreadSource68),
overrideEval68: mark(924 + 11, 924 + 12),
__proto__: protoEval68,
methodEval68(extra) { return this.firstEval68 + this.overrideEval68 + extra; },
get accessEval68() { return this.firstEval68 + 1; },
set accessEval68(value) { setterSinkEval68 = value; },
};
check(orderLog[orderStart68] === 924, "order shorthand setup 68");
check(orderLog[orderStart68 + 1] === 924 + 2, "order computed name 68");
check(orderLog[orderStart68 + 2] === 924 + 6, "order first value 68");
check(orderLog[orderStart68 + 3] === 924 + 8, "order computed value 68");
check(orderLog[orderStart68 + 4] === 924 + 10, "order spread expression 68");
check(orderLog[orderStart68 + 5] === 924 + 11, "order override value 68");
check(evalObj68.firstEval68 === 924 + 7, "first data property 68");
check(evalObj68.shorthandValue68 === 924 + 1, "shorthand data property 68");
check(evalObj68[computedName68] === 924 + 9, "computed data property 68");
check(evalObj68.spreadEval68 === 924 + 3, "spread data property 68");
check(evalObj68.overrideEval68 === 924 + 12, "later property overrides spread 68");
check(Object.getPrototypeOf(evalObj68) === protoEval68, "literal proto setter 68");
check(evalObj68.inheritedEval68 === 924 + 5, "literal proto inherited 68");
check(evalObj68.methodEval68(2) === (924 + 7) + (924 + 12) + 2, "method definition evaluation 68");
check(evalObj68.accessEval68 === 924 + 8, "getter definition evaluation 68");
evalObj68.accessEval68 = 924 + 14;
check(setterSinkEval68 === 924 + 14, "setter definition evaluation 68");

var orderStart69 = orderLog.length;
var shorthandValue69 = mark(937, 937 + 1);
var computedName69 = mark(937 + 2, "computedEval69");
var spreadSource69 = { spreadEval69: 937 + 3, overrideEval69: 937 + 4 };
var protoEval69 = { inheritedEval69: 937 + 5 };
var setterSinkEval69 = 0;
var evalObj69 = {
firstEval69: mark(937 + 6, 937 + 7),
shorthandValue69,
[computedName69]: mark(937 + 8, 937 + 9),
...markObject(937 + 10, spreadSource69),
overrideEval69: mark(937 + 11, 937 + 12),
__proto__: protoEval69,
methodEval69(extra) { return this.firstEval69 + this.overrideEval69 + extra; },
get accessEval69() { return this.firstEval69 + 1; },
set accessEval69(value) { setterSinkEval69 = value; },
};
check(orderLog[orderStart69] === 937, "order shorthand setup 69");
check(orderLog[orderStart69 + 1] === 937 + 2, "order computed name 69");
check(orderLog[orderStart69 + 2] === 937 + 6, "order first value 69");
check(orderLog[orderStart69 + 3] === 937 + 8, "order computed value 69");
check(orderLog[orderStart69 + 4] === 937 + 10, "order spread expression 69");
check(orderLog[orderStart69 + 5] === 937 + 11, "order override value 69");
check(evalObj69.firstEval69 === 937 + 7, "first data property 69");
check(evalObj69.shorthandValue69 === 937 + 1, "shorthand data property 69");
check(evalObj69[computedName69] === 937 + 9, "computed data property 69");
check(evalObj69.spreadEval69 === 937 + 3, "spread data property 69");
check(evalObj69.overrideEval69 === 937 + 12, "later property overrides spread 69");
check(Object.getPrototypeOf(evalObj69) === protoEval69, "literal proto setter 69");
check(evalObj69.inheritedEval69 === 937 + 5, "literal proto inherited 69");
check(evalObj69.methodEval69(2) === (937 + 7) + (937 + 12) + 2, "method definition evaluation 69");
check(evalObj69.accessEval69 === 937 + 8, "getter definition evaluation 69");
evalObj69.accessEval69 = 937 + 14;
check(setterSinkEval69 === 937 + 14, "setter definition evaluation 69");

var orderStart70 = orderLog.length;
var shorthandValue70 = mark(950, 950 + 1);
var computedName70 = mark(950 + 2, "computedEval70");
var spreadSource70 = { spreadEval70: 950 + 3, overrideEval70: 950 + 4 };
var protoEval70 = { inheritedEval70: 950 + 5 };
var setterSinkEval70 = 0;
var evalObj70 = {
firstEval70: mark(950 + 6, 950 + 7),
shorthandValue70,
[computedName70]: mark(950 + 8, 950 + 9),
...markObject(950 + 10, spreadSource70),
overrideEval70: mark(950 + 11, 950 + 12),
__proto__: protoEval70,
methodEval70(extra) { return this.firstEval70 + this.overrideEval70 + extra; },
get accessEval70() { return this.firstEval70 + 1; },
set accessEval70(value) { setterSinkEval70 = value; },
};
check(orderLog[orderStart70] === 950, "order shorthand setup 70");
check(orderLog[orderStart70 + 1] === 950 + 2, "order computed name 70");
check(orderLog[orderStart70 + 2] === 950 + 6, "order first value 70");
check(orderLog[orderStart70 + 3] === 950 + 8, "order computed value 70");
check(orderLog[orderStart70 + 4] === 950 + 10, "order spread expression 70");
check(orderLog[orderStart70 + 5] === 950 + 11, "order override value 70");
check(evalObj70.firstEval70 === 950 + 7, "first data property 70");
check(evalObj70.shorthandValue70 === 950 + 1, "shorthand data property 70");
check(evalObj70[computedName70] === 950 + 9, "computed data property 70");
check(evalObj70.spreadEval70 === 950 + 3, "spread data property 70");
check(evalObj70.overrideEval70 === 950 + 12, "later property overrides spread 70");
check(Object.getPrototypeOf(evalObj70) === protoEval70, "literal proto setter 70");
check(evalObj70.inheritedEval70 === 950 + 5, "literal proto inherited 70");
check(evalObj70.methodEval70(2) === (950 + 7) + (950 + 12) + 2, "method definition evaluation 70");
check(evalObj70.accessEval70 === 950 + 8, "getter definition evaluation 70");
evalObj70.accessEval70 = 950 + 14;
check(setterSinkEval70 === 950 + 14, "setter definition evaluation 70");

var orderStart71 = orderLog.length;
var shorthandValue71 = mark(963, 963 + 1);
var computedName71 = mark(963 + 2, "computedEval71");
var spreadSource71 = { spreadEval71: 963 + 3, overrideEval71: 963 + 4 };
var protoEval71 = { inheritedEval71: 963 + 5 };
var setterSinkEval71 = 0;
var evalObj71 = {
firstEval71: mark(963 + 6, 963 + 7),
shorthandValue71,
[computedName71]: mark(963 + 8, 963 + 9),
...markObject(963 + 10, spreadSource71),
overrideEval71: mark(963 + 11, 963 + 12),
__proto__: protoEval71,
methodEval71(extra) { return this.firstEval71 + this.overrideEval71 + extra; },
get accessEval71() { return this.firstEval71 + 1; },
set accessEval71(value) { setterSinkEval71 = value; },
};
check(orderLog[orderStart71] === 963, "order shorthand setup 71");
check(orderLog[orderStart71 + 1] === 963 + 2, "order computed name 71");
check(orderLog[orderStart71 + 2] === 963 + 6, "order first value 71");
check(orderLog[orderStart71 + 3] === 963 + 8, "order computed value 71");
check(orderLog[orderStart71 + 4] === 963 + 10, "order spread expression 71");
check(orderLog[orderStart71 + 5] === 963 + 11, "order override value 71");
check(evalObj71.firstEval71 === 963 + 7, "first data property 71");
check(evalObj71.shorthandValue71 === 963 + 1, "shorthand data property 71");
check(evalObj71[computedName71] === 963 + 9, "computed data property 71");
check(evalObj71.spreadEval71 === 963 + 3, "spread data property 71");
check(evalObj71.overrideEval71 === 963 + 12, "later property overrides spread 71");
check(Object.getPrototypeOf(evalObj71) === protoEval71, "literal proto setter 71");
check(evalObj71.inheritedEval71 === 963 + 5, "literal proto inherited 71");
check(evalObj71.methodEval71(2) === (963 + 7) + (963 + 12) + 2, "method definition evaluation 71");
check(evalObj71.accessEval71 === 963 + 8, "getter definition evaluation 71");
evalObj71.accessEval71 = 963 + 14;
check(setterSinkEval71 === 963 + 14, "setter definition evaluation 71");

var orderStart72 = orderLog.length;
var shorthandValue72 = mark(976, 976 + 1);
var computedName72 = mark(976 + 2, "computedEval72");
var spreadSource72 = { spreadEval72: 976 + 3, overrideEval72: 976 + 4 };
var protoEval72 = { inheritedEval72: 976 + 5 };
var setterSinkEval72 = 0;
var evalObj72 = {
firstEval72: mark(976 + 6, 976 + 7),
shorthandValue72,
[computedName72]: mark(976 + 8, 976 + 9),
...markObject(976 + 10, spreadSource72),
overrideEval72: mark(976 + 11, 976 + 12),
__proto__: protoEval72,
methodEval72(extra) { return this.firstEval72 + this.overrideEval72 + extra; },
get accessEval72() { return this.firstEval72 + 1; },
set accessEval72(value) { setterSinkEval72 = value; },
};
check(orderLog[orderStart72] === 976, "order shorthand setup 72");
check(orderLog[orderStart72 + 1] === 976 + 2, "order computed name 72");
check(orderLog[orderStart72 + 2] === 976 + 6, "order first value 72");
check(orderLog[orderStart72 + 3] === 976 + 8, "order computed value 72");
check(orderLog[orderStart72 + 4] === 976 + 10, "order spread expression 72");
check(orderLog[orderStart72 + 5] === 976 + 11, "order override value 72");
check(evalObj72.firstEval72 === 976 + 7, "first data property 72");
check(evalObj72.shorthandValue72 === 976 + 1, "shorthand data property 72");
check(evalObj72[computedName72] === 976 + 9, "computed data property 72");
check(evalObj72.spreadEval72 === 976 + 3, "spread data property 72");
check(evalObj72.overrideEval72 === 976 + 12, "later property overrides spread 72");
check(Object.getPrototypeOf(evalObj72) === protoEval72, "literal proto setter 72");
check(evalObj72.inheritedEval72 === 976 + 5, "literal proto inherited 72");
check(evalObj72.methodEval72(2) === (976 + 7) + (976 + 12) + 2, "method definition evaluation 72");
check(evalObj72.accessEval72 === 976 + 8, "getter definition evaluation 72");
evalObj72.accessEval72 = 976 + 14;
check(setterSinkEval72 === 976 + 14, "setter definition evaluation 72");

var orderStart73 = orderLog.length;
var shorthandValue73 = mark(989, 989 + 1);
var computedName73 = mark(989 + 2, "computedEval73");
var spreadSource73 = { spreadEval73: 989 + 3, overrideEval73: 989 + 4 };
var protoEval73 = { inheritedEval73: 989 + 5 };
var setterSinkEval73 = 0;
var evalObj73 = {
firstEval73: mark(989 + 6, 989 + 7),
shorthandValue73,
[computedName73]: mark(989 + 8, 989 + 9),
...markObject(989 + 10, spreadSource73),
overrideEval73: mark(989 + 11, 989 + 12),
__proto__: protoEval73,
methodEval73(extra) { return this.firstEval73 + this.overrideEval73 + extra; },
get accessEval73() { return this.firstEval73 + 1; },
set accessEval73(value) { setterSinkEval73 = value; },
};
check(orderLog[orderStart73] === 989, "order shorthand setup 73");
check(orderLog[orderStart73 + 1] === 989 + 2, "order computed name 73");
check(orderLog[orderStart73 + 2] === 989 + 6, "order first value 73");
check(orderLog[orderStart73 + 3] === 989 + 8, "order computed value 73");
check(orderLog[orderStart73 + 4] === 989 + 10, "order spread expression 73");
check(orderLog[orderStart73 + 5] === 989 + 11, "order override value 73");
check(evalObj73.firstEval73 === 989 + 7, "first data property 73");
check(evalObj73.shorthandValue73 === 989 + 1, "shorthand data property 73");
check(evalObj73[computedName73] === 989 + 9, "computed data property 73");
check(evalObj73.spreadEval73 === 989 + 3, "spread data property 73");
check(evalObj73.overrideEval73 === 989 + 12, "later property overrides spread 73");
check(Object.getPrototypeOf(evalObj73) === protoEval73, "literal proto setter 73");
check(evalObj73.inheritedEval73 === 989 + 5, "literal proto inherited 73");
check(evalObj73.methodEval73(2) === (989 + 7) + (989 + 12) + 2, "method definition evaluation 73");
check(evalObj73.accessEval73 === 989 + 8, "getter definition evaluation 73");
evalObj73.accessEval73 = 989 + 14;
check(setterSinkEval73 === 989 + 14, "setter definition evaluation 73");

var orderStart74 = orderLog.length;
var shorthandValue74 = mark(1002, 1002 + 1);
var computedName74 = mark(1002 + 2, "computedEval74");
var spreadSource74 = { spreadEval74: 1002 + 3, overrideEval74: 1002 + 4 };
var protoEval74 = { inheritedEval74: 1002 + 5 };
var setterSinkEval74 = 0;
var evalObj74 = {
firstEval74: mark(1002 + 6, 1002 + 7),
shorthandValue74,
[computedName74]: mark(1002 + 8, 1002 + 9),
...markObject(1002 + 10, spreadSource74),
overrideEval74: mark(1002 + 11, 1002 + 12),
__proto__: protoEval74,
methodEval74(extra) { return this.firstEval74 + this.overrideEval74 + extra; },
get accessEval74() { return this.firstEval74 + 1; },
set accessEval74(value) { setterSinkEval74 = value; },
};
check(orderLog[orderStart74] === 1002, "order shorthand setup 74");
check(orderLog[orderStart74 + 1] === 1002 + 2, "order computed name 74");
check(orderLog[orderStart74 + 2] === 1002 + 6, "order first value 74");
check(orderLog[orderStart74 + 3] === 1002 + 8, "order computed value 74");
check(orderLog[orderStart74 + 4] === 1002 + 10, "order spread expression 74");
check(orderLog[orderStart74 + 5] === 1002 + 11, "order override value 74");
check(evalObj74.firstEval74 === 1002 + 7, "first data property 74");
check(evalObj74.shorthandValue74 === 1002 + 1, "shorthand data property 74");
check(evalObj74[computedName74] === 1002 + 9, "computed data property 74");
check(evalObj74.spreadEval74 === 1002 + 3, "spread data property 74");
check(evalObj74.overrideEval74 === 1002 + 12, "later property overrides spread 74");
check(Object.getPrototypeOf(evalObj74) === protoEval74, "literal proto setter 74");
check(evalObj74.inheritedEval74 === 1002 + 5, "literal proto inherited 74");
check(evalObj74.methodEval74(2) === (1002 + 7) + (1002 + 12) + 2, "method definition evaluation 74");
check(evalObj74.accessEval74 === 1002 + 8, "getter definition evaluation 74");
evalObj74.accessEval74 = 1002 + 14;
check(setterSinkEval74 === 1002 + 14, "setter definition evaluation 74");

var orderStart75 = orderLog.length;
var shorthandValue75 = mark(1015, 1015 + 1);
var computedName75 = mark(1015 + 2, "computedEval75");
var spreadSource75 = { spreadEval75: 1015 + 3, overrideEval75: 1015 + 4 };
var protoEval75 = { inheritedEval75: 1015 + 5 };
var setterSinkEval75 = 0;
var evalObj75 = {
firstEval75: mark(1015 + 6, 1015 + 7),
shorthandValue75,
[computedName75]: mark(1015 + 8, 1015 + 9),
...markObject(1015 + 10, spreadSource75),
overrideEval75: mark(1015 + 11, 1015 + 12),
__proto__: protoEval75,
methodEval75(extra) { return this.firstEval75 + this.overrideEval75 + extra; },
get accessEval75() { return this.firstEval75 + 1; },
set accessEval75(value) { setterSinkEval75 = value; },
};
check(orderLog[orderStart75] === 1015, "order shorthand setup 75");
check(orderLog[orderStart75 + 1] === 1015 + 2, "order computed name 75");
check(orderLog[orderStart75 + 2] === 1015 + 6, "order first value 75");
check(orderLog[orderStart75 + 3] === 1015 + 8, "order computed value 75");
check(orderLog[orderStart75 + 4] === 1015 + 10, "order spread expression 75");
check(orderLog[orderStart75 + 5] === 1015 + 11, "order override value 75");
check(evalObj75.firstEval75 === 1015 + 7, "first data property 75");
check(evalObj75.shorthandValue75 === 1015 + 1, "shorthand data property 75");
check(evalObj75[computedName75] === 1015 + 9, "computed data property 75");
check(evalObj75.spreadEval75 === 1015 + 3, "spread data property 75");
check(evalObj75.overrideEval75 === 1015 + 12, "later property overrides spread 75");
check(Object.getPrototypeOf(evalObj75) === protoEval75, "literal proto setter 75");
check(evalObj75.inheritedEval75 === 1015 + 5, "literal proto inherited 75");
check(evalObj75.methodEval75(2) === (1015 + 7) + (1015 + 12) + 2, "method definition evaluation 75");
check(evalObj75.accessEval75 === 1015 + 8, "getter definition evaluation 75");
evalObj75.accessEval75 = 1015 + 14;
check(setterSinkEval75 === 1015 + 14, "setter definition evaluation 75");

var orderStart76 = orderLog.length;
var shorthandValue76 = mark(1028, 1028 + 1);
var computedName76 = mark(1028 + 2, "computedEval76");
var spreadSource76 = { spreadEval76: 1028 + 3, overrideEval76: 1028 + 4 };
var protoEval76 = { inheritedEval76: 1028 + 5 };
var setterSinkEval76 = 0;
var evalObj76 = {
firstEval76: mark(1028 + 6, 1028 + 7),
shorthandValue76,
[computedName76]: mark(1028 + 8, 1028 + 9),
...markObject(1028 + 10, spreadSource76),
overrideEval76: mark(1028 + 11, 1028 + 12),
__proto__: protoEval76,
methodEval76(extra) { return this.firstEval76 + this.overrideEval76 + extra; },
get accessEval76() { return this.firstEval76 + 1; },
set accessEval76(value) { setterSinkEval76 = value; },
};
check(orderLog[orderStart76] === 1028, "order shorthand setup 76");
check(orderLog[orderStart76 + 1] === 1028 + 2, "order computed name 76");
check(orderLog[orderStart76 + 2] === 1028 + 6, "order first value 76");
check(orderLog[orderStart76 + 3] === 1028 + 8, "order computed value 76");
check(orderLog[orderStart76 + 4] === 1028 + 10, "order spread expression 76");
check(orderLog[orderStart76 + 5] === 1028 + 11, "order override value 76");
check(evalObj76.firstEval76 === 1028 + 7, "first data property 76");
check(evalObj76.shorthandValue76 === 1028 + 1, "shorthand data property 76");
check(evalObj76[computedName76] === 1028 + 9, "computed data property 76");
check(evalObj76.spreadEval76 === 1028 + 3, "spread data property 76");
check(evalObj76.overrideEval76 === 1028 + 12, "later property overrides spread 76");
check(Object.getPrototypeOf(evalObj76) === protoEval76, "literal proto setter 76");
check(evalObj76.inheritedEval76 === 1028 + 5, "literal proto inherited 76");
check(evalObj76.methodEval76(2) === (1028 + 7) + (1028 + 12) + 2, "method definition evaluation 76");
check(evalObj76.accessEval76 === 1028 + 8, "getter definition evaluation 76");
evalObj76.accessEval76 = 1028 + 14;
check(setterSinkEval76 === 1028 + 14, "setter definition evaluation 76");

var orderStart77 = orderLog.length;
var shorthandValue77 = mark(1041, 1041 + 1);
var computedName77 = mark(1041 + 2, "computedEval77");
var spreadSource77 = { spreadEval77: 1041 + 3, overrideEval77: 1041 + 4 };
var protoEval77 = { inheritedEval77: 1041 + 5 };
var setterSinkEval77 = 0;
var evalObj77 = {
firstEval77: mark(1041 + 6, 1041 + 7),
shorthandValue77,
[computedName77]: mark(1041 + 8, 1041 + 9),
...markObject(1041 + 10, spreadSource77),
overrideEval77: mark(1041 + 11, 1041 + 12),
__proto__: protoEval77,
methodEval77(extra) { return this.firstEval77 + this.overrideEval77 + extra; },
get accessEval77() { return this.firstEval77 + 1; },
set accessEval77(value) { setterSinkEval77 = value; },
};
check(orderLog[orderStart77] === 1041, "order shorthand setup 77");
check(orderLog[orderStart77 + 1] === 1041 + 2, "order computed name 77");
check(orderLog[orderStart77 + 2] === 1041 + 6, "order first value 77");
check(orderLog[orderStart77 + 3] === 1041 + 8, "order computed value 77");
check(orderLog[orderStart77 + 4] === 1041 + 10, "order spread expression 77");
check(orderLog[orderStart77 + 5] === 1041 + 11, "order override value 77");
check(evalObj77.firstEval77 === 1041 + 7, "first data property 77");
check(evalObj77.shorthandValue77 === 1041 + 1, "shorthand data property 77");
check(evalObj77[computedName77] === 1041 + 9, "computed data property 77");
check(evalObj77.spreadEval77 === 1041 + 3, "spread data property 77");
check(evalObj77.overrideEval77 === 1041 + 12, "later property overrides spread 77");
check(Object.getPrototypeOf(evalObj77) === protoEval77, "literal proto setter 77");
check(evalObj77.inheritedEval77 === 1041 + 5, "literal proto inherited 77");
check(evalObj77.methodEval77(2) === (1041 + 7) + (1041 + 12) + 2, "method definition evaluation 77");
check(evalObj77.accessEval77 === 1041 + 8, "getter definition evaluation 77");
evalObj77.accessEval77 = 1041 + 14;
check(setterSinkEval77 === 1041 + 14, "setter definition evaluation 77");

var orderStart78 = orderLog.length;
var shorthandValue78 = mark(1054, 1054 + 1);
var computedName78 = mark(1054 + 2, "computedEval78");
var spreadSource78 = { spreadEval78: 1054 + 3, overrideEval78: 1054 + 4 };
var protoEval78 = { inheritedEval78: 1054 + 5 };
var setterSinkEval78 = 0;
var evalObj78 = {
firstEval78: mark(1054 + 6, 1054 + 7),
shorthandValue78,
[computedName78]: mark(1054 + 8, 1054 + 9),
...markObject(1054 + 10, spreadSource78),
overrideEval78: mark(1054 + 11, 1054 + 12),
__proto__: protoEval78,
methodEval78(extra) { return this.firstEval78 + this.overrideEval78 + extra; },
get accessEval78() { return this.firstEval78 + 1; },
set accessEval78(value) { setterSinkEval78 = value; },
};
check(orderLog[orderStart78] === 1054, "order shorthand setup 78");
check(orderLog[orderStart78 + 1] === 1054 + 2, "order computed name 78");
check(orderLog[orderStart78 + 2] === 1054 + 6, "order first value 78");
check(orderLog[orderStart78 + 3] === 1054 + 8, "order computed value 78");
check(orderLog[orderStart78 + 4] === 1054 + 10, "order spread expression 78");
check(orderLog[orderStart78 + 5] === 1054 + 11, "order override value 78");
check(evalObj78.firstEval78 === 1054 + 7, "first data property 78");
check(evalObj78.shorthandValue78 === 1054 + 1, "shorthand data property 78");
check(evalObj78[computedName78] === 1054 + 9, "computed data property 78");
check(evalObj78.spreadEval78 === 1054 + 3, "spread data property 78");
check(evalObj78.overrideEval78 === 1054 + 12, "later property overrides spread 78");
check(Object.getPrototypeOf(evalObj78) === protoEval78, "literal proto setter 78");
check(evalObj78.inheritedEval78 === 1054 + 5, "literal proto inherited 78");
check(evalObj78.methodEval78(2) === (1054 + 7) + (1054 + 12) + 2, "method definition evaluation 78");
check(evalObj78.accessEval78 === 1054 + 8, "getter definition evaluation 78");
evalObj78.accessEval78 = 1054 + 14;
check(setterSinkEval78 === 1054 + 14, "setter definition evaluation 78");

var orderStart79 = orderLog.length;
var shorthandValue79 = mark(1067, 1067 + 1);
var computedName79 = mark(1067 + 2, "computedEval79");
var spreadSource79 = { spreadEval79: 1067 + 3, overrideEval79: 1067 + 4 };
var protoEval79 = { inheritedEval79: 1067 + 5 };
var setterSinkEval79 = 0;
var evalObj79 = {
firstEval79: mark(1067 + 6, 1067 + 7),
shorthandValue79,
[computedName79]: mark(1067 + 8, 1067 + 9),
...markObject(1067 + 10, spreadSource79),
overrideEval79: mark(1067 + 11, 1067 + 12),
__proto__: protoEval79,
methodEval79(extra) { return this.firstEval79 + this.overrideEval79 + extra; },
get accessEval79() { return this.firstEval79 + 1; },
set accessEval79(value) { setterSinkEval79 = value; },
};
check(orderLog[orderStart79] === 1067, "order shorthand setup 79");
check(orderLog[orderStart79 + 1] === 1067 + 2, "order computed name 79");
check(orderLog[orderStart79 + 2] === 1067 + 6, "order first value 79");
check(orderLog[orderStart79 + 3] === 1067 + 8, "order computed value 79");
check(orderLog[orderStart79 + 4] === 1067 + 10, "order spread expression 79");
check(orderLog[orderStart79 + 5] === 1067 + 11, "order override value 79");
check(evalObj79.firstEval79 === 1067 + 7, "first data property 79");
check(evalObj79.shorthandValue79 === 1067 + 1, "shorthand data property 79");
check(evalObj79[computedName79] === 1067 + 9, "computed data property 79");
check(evalObj79.spreadEval79 === 1067 + 3, "spread data property 79");
check(evalObj79.overrideEval79 === 1067 + 12, "later property overrides spread 79");
check(Object.getPrototypeOf(evalObj79) === protoEval79, "literal proto setter 79");
check(evalObj79.inheritedEval79 === 1067 + 5, "literal proto inherited 79");
check(evalObj79.methodEval79(2) === (1067 + 7) + (1067 + 12) + 2, "method definition evaluation 79");
check(evalObj79.accessEval79 === 1067 + 8, "getter definition evaluation 79");
evalObj79.accessEval79 = 1067 + 14;
check(setterSinkEval79 === 1067 + 14, "setter definition evaluation 79");

var orderStart80 = orderLog.length;
var shorthandValue80 = mark(1080, 1080 + 1);
var computedName80 = mark(1080 + 2, "computedEval80");
var spreadSource80 = { spreadEval80: 1080 + 3, overrideEval80: 1080 + 4 };
var protoEval80 = { inheritedEval80: 1080 + 5 };
var setterSinkEval80 = 0;
var evalObj80 = {
firstEval80: mark(1080 + 6, 1080 + 7),
shorthandValue80,
[computedName80]: mark(1080 + 8, 1080 + 9),
...markObject(1080 + 10, spreadSource80),
overrideEval80: mark(1080 + 11, 1080 + 12),
__proto__: protoEval80,
methodEval80(extra) { return this.firstEval80 + this.overrideEval80 + extra; },
get accessEval80() { return this.firstEval80 + 1; },
set accessEval80(value) { setterSinkEval80 = value; },
};
check(orderLog[orderStart80] === 1080, "order shorthand setup 80");
check(orderLog[orderStart80 + 1] === 1080 + 2, "order computed name 80");
check(orderLog[orderStart80 + 2] === 1080 + 6, "order first value 80");
check(orderLog[orderStart80 + 3] === 1080 + 8, "order computed value 80");
check(orderLog[orderStart80 + 4] === 1080 + 10, "order spread expression 80");
check(orderLog[orderStart80 + 5] === 1080 + 11, "order override value 80");
check(evalObj80.firstEval80 === 1080 + 7, "first data property 80");
check(evalObj80.shorthandValue80 === 1080 + 1, "shorthand data property 80");
check(evalObj80[computedName80] === 1080 + 9, "computed data property 80");
check(evalObj80.spreadEval80 === 1080 + 3, "spread data property 80");
check(evalObj80.overrideEval80 === 1080 + 12, "later property overrides spread 80");
check(Object.getPrototypeOf(evalObj80) === protoEval80, "literal proto setter 80");
check(evalObj80.inheritedEval80 === 1080 + 5, "literal proto inherited 80");
check(evalObj80.methodEval80(2) === (1080 + 7) + (1080 + 12) + 2, "method definition evaluation 80");
check(evalObj80.accessEval80 === 1080 + 8, "getter definition evaluation 80");
evalObj80.accessEval80 = 1080 + 14;
check(setterSinkEval80 === 1080 + 14, "setter definition evaluation 80");

var orderStart81 = orderLog.length;
var shorthandValue81 = mark(1093, 1093 + 1);
var computedName81 = mark(1093 + 2, "computedEval81");
var spreadSource81 = { spreadEval81: 1093 + 3, overrideEval81: 1093 + 4 };
var protoEval81 = { inheritedEval81: 1093 + 5 };
var setterSinkEval81 = 0;
var evalObj81 = {
firstEval81: mark(1093 + 6, 1093 + 7),
shorthandValue81,
[computedName81]: mark(1093 + 8, 1093 + 9),
...markObject(1093 + 10, spreadSource81),
overrideEval81: mark(1093 + 11, 1093 + 12),
__proto__: protoEval81,
methodEval81(extra) { return this.firstEval81 + this.overrideEval81 + extra; },
get accessEval81() { return this.firstEval81 + 1; },
set accessEval81(value) { setterSinkEval81 = value; },
};
check(orderLog[orderStart81] === 1093, "order shorthand setup 81");
check(orderLog[orderStart81 + 1] === 1093 + 2, "order computed name 81");
check(orderLog[orderStart81 + 2] === 1093 + 6, "order first value 81");
check(orderLog[orderStart81 + 3] === 1093 + 8, "order computed value 81");
check(orderLog[orderStart81 + 4] === 1093 + 10, "order spread expression 81");
check(orderLog[orderStart81 + 5] === 1093 + 11, "order override value 81");
check(evalObj81.firstEval81 === 1093 + 7, "first data property 81");
check(evalObj81.shorthandValue81 === 1093 + 1, "shorthand data property 81");
check(evalObj81[computedName81] === 1093 + 9, "computed data property 81");
check(evalObj81.spreadEval81 === 1093 + 3, "spread data property 81");
check(evalObj81.overrideEval81 === 1093 + 12, "later property overrides spread 81");
check(Object.getPrototypeOf(evalObj81) === protoEval81, "literal proto setter 81");
check(evalObj81.inheritedEval81 === 1093 + 5, "literal proto inherited 81");
check(evalObj81.methodEval81(2) === (1093 + 7) + (1093 + 12) + 2, "method definition evaluation 81");
check(evalObj81.accessEval81 === 1093 + 8, "getter definition evaluation 81");
evalObj81.accessEval81 = 1093 + 14;
check(setterSinkEval81 === 1093 + 14, "setter definition evaluation 81");

var orderStart82 = orderLog.length;
var shorthandValue82 = mark(1106, 1106 + 1);
var computedName82 = mark(1106 + 2, "computedEval82");
var spreadSource82 = { spreadEval82: 1106 + 3, overrideEval82: 1106 + 4 };
var protoEval82 = { inheritedEval82: 1106 + 5 };
var setterSinkEval82 = 0;
var evalObj82 = {
firstEval82: mark(1106 + 6, 1106 + 7),
shorthandValue82,
[computedName82]: mark(1106 + 8, 1106 + 9),
...markObject(1106 + 10, spreadSource82),
overrideEval82: mark(1106 + 11, 1106 + 12),
__proto__: protoEval82,
methodEval82(extra) { return this.firstEval82 + this.overrideEval82 + extra; },
get accessEval82() { return this.firstEval82 + 1; },
set accessEval82(value) { setterSinkEval82 = value; },
};
check(orderLog[orderStart82] === 1106, "order shorthand setup 82");
check(orderLog[orderStart82 + 1] === 1106 + 2, "order computed name 82");
check(orderLog[orderStart82 + 2] === 1106 + 6, "order first value 82");
check(orderLog[orderStart82 + 3] === 1106 + 8, "order computed value 82");
check(orderLog[orderStart82 + 4] === 1106 + 10, "order spread expression 82");
check(orderLog[orderStart82 + 5] === 1106 + 11, "order override value 82");
check(evalObj82.firstEval82 === 1106 + 7, "first data property 82");
check(evalObj82.shorthandValue82 === 1106 + 1, "shorthand data property 82");
check(evalObj82[computedName82] === 1106 + 9, "computed data property 82");
check(evalObj82.spreadEval82 === 1106 + 3, "spread data property 82");
check(evalObj82.overrideEval82 === 1106 + 12, "later property overrides spread 82");
check(Object.getPrototypeOf(evalObj82) === protoEval82, "literal proto setter 82");
check(evalObj82.inheritedEval82 === 1106 + 5, "literal proto inherited 82");
check(evalObj82.methodEval82(2) === (1106 + 7) + (1106 + 12) + 2, "method definition evaluation 82");
check(evalObj82.accessEval82 === 1106 + 8, "getter definition evaluation 82");
evalObj82.accessEval82 = 1106 + 14;
check(setterSinkEval82 === 1106 + 14, "setter definition evaluation 82");

var orderStart83 = orderLog.length;
var shorthandValue83 = mark(1119, 1119 + 1);
var computedName83 = mark(1119 + 2, "computedEval83");
var spreadSource83 = { spreadEval83: 1119 + 3, overrideEval83: 1119 + 4 };
var protoEval83 = { inheritedEval83: 1119 + 5 };
var setterSinkEval83 = 0;
var evalObj83 = {
firstEval83: mark(1119 + 6, 1119 + 7),
shorthandValue83,
[computedName83]: mark(1119 + 8, 1119 + 9),
...markObject(1119 + 10, spreadSource83),
overrideEval83: mark(1119 + 11, 1119 + 12),
__proto__: protoEval83,
methodEval83(extra) { return this.firstEval83 + this.overrideEval83 + extra; },
get accessEval83() { return this.firstEval83 + 1; },
set accessEval83(value) { setterSinkEval83 = value; },
};
check(orderLog[orderStart83] === 1119, "order shorthand setup 83");
check(orderLog[orderStart83 + 1] === 1119 + 2, "order computed name 83");
check(orderLog[orderStart83 + 2] === 1119 + 6, "order first value 83");
check(orderLog[orderStart83 + 3] === 1119 + 8, "order computed value 83");
check(orderLog[orderStart83 + 4] === 1119 + 10, "order spread expression 83");
check(orderLog[orderStart83 + 5] === 1119 + 11, "order override value 83");
check(evalObj83.firstEval83 === 1119 + 7, "first data property 83");
check(evalObj83.shorthandValue83 === 1119 + 1, "shorthand data property 83");
check(evalObj83[computedName83] === 1119 + 9, "computed data property 83");
check(evalObj83.spreadEval83 === 1119 + 3, "spread data property 83");
check(evalObj83.overrideEval83 === 1119 + 12, "later property overrides spread 83");
check(Object.getPrototypeOf(evalObj83) === protoEval83, "literal proto setter 83");
check(evalObj83.inheritedEval83 === 1119 + 5, "literal proto inherited 83");
check(evalObj83.methodEval83(2) === (1119 + 7) + (1119 + 12) + 2, "method definition evaluation 83");
check(evalObj83.accessEval83 === 1119 + 8, "getter definition evaluation 83");
evalObj83.accessEval83 = 1119 + 14;
check(setterSinkEval83 === 1119 + 14, "setter definition evaluation 83");

var orderStart84 = orderLog.length;
var shorthandValue84 = mark(1132, 1132 + 1);
var computedName84 = mark(1132 + 2, "computedEval84");
var spreadSource84 = { spreadEval84: 1132 + 3, overrideEval84: 1132 + 4 };
var protoEval84 = { inheritedEval84: 1132 + 5 };
var setterSinkEval84 = 0;
var evalObj84 = {
firstEval84: mark(1132 + 6, 1132 + 7),
shorthandValue84,
[computedName84]: mark(1132 + 8, 1132 + 9),
...markObject(1132 + 10, spreadSource84),
overrideEval84: mark(1132 + 11, 1132 + 12),
__proto__: protoEval84,
methodEval84(extra) { return this.firstEval84 + this.overrideEval84 + extra; },
get accessEval84() { return this.firstEval84 + 1; },
set accessEval84(value) { setterSinkEval84 = value; },
};
check(orderLog[orderStart84] === 1132, "order shorthand setup 84");
check(orderLog[orderStart84 + 1] === 1132 + 2, "order computed name 84");
check(orderLog[orderStart84 + 2] === 1132 + 6, "order first value 84");
check(orderLog[orderStart84 + 3] === 1132 + 8, "order computed value 84");
check(orderLog[orderStart84 + 4] === 1132 + 10, "order spread expression 84");
check(orderLog[orderStart84 + 5] === 1132 + 11, "order override value 84");
check(evalObj84.firstEval84 === 1132 + 7, "first data property 84");
check(evalObj84.shorthandValue84 === 1132 + 1, "shorthand data property 84");
check(evalObj84[computedName84] === 1132 + 9, "computed data property 84");
check(evalObj84.spreadEval84 === 1132 + 3, "spread data property 84");
check(evalObj84.overrideEval84 === 1132 + 12, "later property overrides spread 84");
check(Object.getPrototypeOf(evalObj84) === protoEval84, "literal proto setter 84");
check(evalObj84.inheritedEval84 === 1132 + 5, "literal proto inherited 84");
check(evalObj84.methodEval84(2) === (1132 + 7) + (1132 + 12) + 2, "method definition evaluation 84");
check(evalObj84.accessEval84 === 1132 + 8, "getter definition evaluation 84");
evalObj84.accessEval84 = 1132 + 14;
check(setterSinkEval84 === 1132 + 14, "setter definition evaluation 84");

var orderStart85 = orderLog.length;
var shorthandValue85 = mark(1145, 1145 + 1);
var computedName85 = mark(1145 + 2, "computedEval85");
var spreadSource85 = { spreadEval85: 1145 + 3, overrideEval85: 1145 + 4 };
var protoEval85 = { inheritedEval85: 1145 + 5 };
var setterSinkEval85 = 0;
var evalObj85 = {
firstEval85: mark(1145 + 6, 1145 + 7),
shorthandValue85,
[computedName85]: mark(1145 + 8, 1145 + 9),
...markObject(1145 + 10, spreadSource85),
overrideEval85: mark(1145 + 11, 1145 + 12),
__proto__: protoEval85,
methodEval85(extra) { return this.firstEval85 + this.overrideEval85 + extra; },
get accessEval85() { return this.firstEval85 + 1; },
set accessEval85(value) { setterSinkEval85 = value; },
};
check(orderLog[orderStart85] === 1145, "order shorthand setup 85");
check(orderLog[orderStart85 + 1] === 1145 + 2, "order computed name 85");
check(orderLog[orderStart85 + 2] === 1145 + 6, "order first value 85");
check(orderLog[orderStart85 + 3] === 1145 + 8, "order computed value 85");
check(orderLog[orderStart85 + 4] === 1145 + 10, "order spread expression 85");
check(orderLog[orderStart85 + 5] === 1145 + 11, "order override value 85");
check(evalObj85.firstEval85 === 1145 + 7, "first data property 85");
check(evalObj85.shorthandValue85 === 1145 + 1, "shorthand data property 85");
check(evalObj85[computedName85] === 1145 + 9, "computed data property 85");
check(evalObj85.spreadEval85 === 1145 + 3, "spread data property 85");
check(evalObj85.overrideEval85 === 1145 + 12, "later property overrides spread 85");
check(Object.getPrototypeOf(evalObj85) === protoEval85, "literal proto setter 85");
check(evalObj85.inheritedEval85 === 1145 + 5, "literal proto inherited 85");
check(evalObj85.methodEval85(2) === (1145 + 7) + (1145 + 12) + 2, "method definition evaluation 85");
check(evalObj85.accessEval85 === 1145 + 8, "getter definition evaluation 85");
evalObj85.accessEval85 = 1145 + 14;
check(setterSinkEval85 === 1145 + 14, "setter definition evaluation 85");

var orderStart86 = orderLog.length;
var shorthandValue86 = mark(1158, 1158 + 1);
var computedName86 = mark(1158 + 2, "computedEval86");
var spreadSource86 = { spreadEval86: 1158 + 3, overrideEval86: 1158 + 4 };
var protoEval86 = { inheritedEval86: 1158 + 5 };
var setterSinkEval86 = 0;
var evalObj86 = {
firstEval86: mark(1158 + 6, 1158 + 7),
shorthandValue86,
[computedName86]: mark(1158 + 8, 1158 + 9),
...markObject(1158 + 10, spreadSource86),
overrideEval86: mark(1158 + 11, 1158 + 12),
__proto__: protoEval86,
methodEval86(extra) { return this.firstEval86 + this.overrideEval86 + extra; },
get accessEval86() { return this.firstEval86 + 1; },
set accessEval86(value) { setterSinkEval86 = value; },
};
check(orderLog[orderStart86] === 1158, "order shorthand setup 86");
check(orderLog[orderStart86 + 1] === 1158 + 2, "order computed name 86");
check(orderLog[orderStart86 + 2] === 1158 + 6, "order first value 86");
check(orderLog[orderStart86 + 3] === 1158 + 8, "order computed value 86");
check(orderLog[orderStart86 + 4] === 1158 + 10, "order spread expression 86");
check(orderLog[orderStart86 + 5] === 1158 + 11, "order override value 86");
check(evalObj86.firstEval86 === 1158 + 7, "first data property 86");
check(evalObj86.shorthandValue86 === 1158 + 1, "shorthand data property 86");
check(evalObj86[computedName86] === 1158 + 9, "computed data property 86");
check(evalObj86.spreadEval86 === 1158 + 3, "spread data property 86");
check(evalObj86.overrideEval86 === 1158 + 12, "later property overrides spread 86");
check(Object.getPrototypeOf(evalObj86) === protoEval86, "literal proto setter 86");
check(evalObj86.inheritedEval86 === 1158 + 5, "literal proto inherited 86");
check(evalObj86.methodEval86(2) === (1158 + 7) + (1158 + 12) + 2, "method definition evaluation 86");
check(evalObj86.accessEval86 === 1158 + 8, "getter definition evaluation 86");
evalObj86.accessEval86 = 1158 + 14;
check(setterSinkEval86 === 1158 + 14, "setter definition evaluation 86");

var orderStart87 = orderLog.length;
var shorthandValue87 = mark(1171, 1171 + 1);
var computedName87 = mark(1171 + 2, "computedEval87");
var spreadSource87 = { spreadEval87: 1171 + 3, overrideEval87: 1171 + 4 };
var protoEval87 = { inheritedEval87: 1171 + 5 };
var setterSinkEval87 = 0;
var evalObj87 = {
firstEval87: mark(1171 + 6, 1171 + 7),
shorthandValue87,
[computedName87]: mark(1171 + 8, 1171 + 9),
...markObject(1171 + 10, spreadSource87),
overrideEval87: mark(1171 + 11, 1171 + 12),
__proto__: protoEval87,
methodEval87(extra) { return this.firstEval87 + this.overrideEval87 + extra; },
get accessEval87() { return this.firstEval87 + 1; },
set accessEval87(value) { setterSinkEval87 = value; },
};
check(orderLog[orderStart87] === 1171, "order shorthand setup 87");
check(orderLog[orderStart87 + 1] === 1171 + 2, "order computed name 87");
check(orderLog[orderStart87 + 2] === 1171 + 6, "order first value 87");
check(orderLog[orderStart87 + 3] === 1171 + 8, "order computed value 87");
check(orderLog[orderStart87 + 4] === 1171 + 10, "order spread expression 87");
check(orderLog[orderStart87 + 5] === 1171 + 11, "order override value 87");
check(evalObj87.firstEval87 === 1171 + 7, "first data property 87");
check(evalObj87.shorthandValue87 === 1171 + 1, "shorthand data property 87");
check(evalObj87[computedName87] === 1171 + 9, "computed data property 87");
check(evalObj87.spreadEval87 === 1171 + 3, "spread data property 87");
check(evalObj87.overrideEval87 === 1171 + 12, "later property overrides spread 87");
check(Object.getPrototypeOf(evalObj87) === protoEval87, "literal proto setter 87");
check(evalObj87.inheritedEval87 === 1171 + 5, "literal proto inherited 87");
check(evalObj87.methodEval87(2) === (1171 + 7) + (1171 + 12) + 2, "method definition evaluation 87");
check(evalObj87.accessEval87 === 1171 + 8, "getter definition evaluation 87");
evalObj87.accessEval87 = 1171 + 14;
check(setterSinkEval87 === 1171 + 14, "setter definition evaluation 87");

var orderStart88 = orderLog.length;
var shorthandValue88 = mark(1184, 1184 + 1);
var computedName88 = mark(1184 + 2, "computedEval88");
var spreadSource88 = { spreadEval88: 1184 + 3, overrideEval88: 1184 + 4 };
var protoEval88 = { inheritedEval88: 1184 + 5 };
var setterSinkEval88 = 0;
var evalObj88 = {
firstEval88: mark(1184 + 6, 1184 + 7),
shorthandValue88,
[computedName88]: mark(1184 + 8, 1184 + 9),
...markObject(1184 + 10, spreadSource88),
overrideEval88: mark(1184 + 11, 1184 + 12),
__proto__: protoEval88,
methodEval88(extra) { return this.firstEval88 + this.overrideEval88 + extra; },
get accessEval88() { return this.firstEval88 + 1; },
set accessEval88(value) { setterSinkEval88 = value; },
};
check(orderLog[orderStart88] === 1184, "order shorthand setup 88");
check(orderLog[orderStart88 + 1] === 1184 + 2, "order computed name 88");
check(orderLog[orderStart88 + 2] === 1184 + 6, "order first value 88");
check(orderLog[orderStart88 + 3] === 1184 + 8, "order computed value 88");
check(orderLog[orderStart88 + 4] === 1184 + 10, "order spread expression 88");
check(orderLog[orderStart88 + 5] === 1184 + 11, "order override value 88");
check(evalObj88.firstEval88 === 1184 + 7, "first data property 88");
check(evalObj88.shorthandValue88 === 1184 + 1, "shorthand data property 88");
check(evalObj88[computedName88] === 1184 + 9, "computed data property 88");
check(evalObj88.spreadEval88 === 1184 + 3, "spread data property 88");
check(evalObj88.overrideEval88 === 1184 + 12, "later property overrides spread 88");
check(Object.getPrototypeOf(evalObj88) === protoEval88, "literal proto setter 88");
check(evalObj88.inheritedEval88 === 1184 + 5, "literal proto inherited 88");
check(evalObj88.methodEval88(2) === (1184 + 7) + (1184 + 12) + 2, "method definition evaluation 88");
check(evalObj88.accessEval88 === 1184 + 8, "getter definition evaluation 88");
evalObj88.accessEval88 = 1184 + 14;
check(setterSinkEval88 === 1184 + 14, "setter definition evaluation 88");

var orderStart89 = orderLog.length;
var shorthandValue89 = mark(1197, 1197 + 1);
var computedName89 = mark(1197 + 2, "computedEval89");
var spreadSource89 = { spreadEval89: 1197 + 3, overrideEval89: 1197 + 4 };
var protoEval89 = { inheritedEval89: 1197 + 5 };
var setterSinkEval89 = 0;
var evalObj89 = {
firstEval89: mark(1197 + 6, 1197 + 7),
shorthandValue89,
[computedName89]: mark(1197 + 8, 1197 + 9),
...markObject(1197 + 10, spreadSource89),
overrideEval89: mark(1197 + 11, 1197 + 12),
__proto__: protoEval89,
methodEval89(extra) { return this.firstEval89 + this.overrideEval89 + extra; },
get accessEval89() { return this.firstEval89 + 1; },
set accessEval89(value) { setterSinkEval89 = value; },
};
check(orderLog[orderStart89] === 1197, "order shorthand setup 89");
check(orderLog[orderStart89 + 1] === 1197 + 2, "order computed name 89");
check(orderLog[orderStart89 + 2] === 1197 + 6, "order first value 89");
check(orderLog[orderStart89 + 3] === 1197 + 8, "order computed value 89");
check(orderLog[orderStart89 + 4] === 1197 + 10, "order spread expression 89");
check(orderLog[orderStart89 + 5] === 1197 + 11, "order override value 89");
check(evalObj89.firstEval89 === 1197 + 7, "first data property 89");
check(evalObj89.shorthandValue89 === 1197 + 1, "shorthand data property 89");
check(evalObj89[computedName89] === 1197 + 9, "computed data property 89");
check(evalObj89.spreadEval89 === 1197 + 3, "spread data property 89");
check(evalObj89.overrideEval89 === 1197 + 12, "later property overrides spread 89");
check(Object.getPrototypeOf(evalObj89) === protoEval89, "literal proto setter 89");
check(evalObj89.inheritedEval89 === 1197 + 5, "literal proto inherited 89");
check(evalObj89.methodEval89(2) === (1197 + 7) + (1197 + 12) + 2, "method definition evaluation 89");
check(evalObj89.accessEval89 === 1197 + 8, "getter definition evaluation 89");
evalObj89.accessEval89 = 1197 + 14;
check(setterSinkEval89 === 1197 + 14, "setter definition evaluation 89");

check(score > 0, "property definition evaluation score");
