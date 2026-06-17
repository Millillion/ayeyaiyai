// behavior: property-definitions-evaluate-and-create-properties
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

check(score > 0, "property definition evaluation score");
