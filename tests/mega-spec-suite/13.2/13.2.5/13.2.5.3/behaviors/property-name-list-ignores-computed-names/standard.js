// behavior: property-name-list-ignores-computed-names
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

var protoList0 = { inheritedList: 20 };
var firstComputedProto0 = { data: 20 + 1 };
var secondComputedProto0 = { data: 20 + 2 };
var listObj0 = { ["__proto__"]: firstComputedProto0, __proto__: protoList0, ["__proto__"]: secondComputedProto0, value0: 20 + 3 };
check(Object.getPrototypeOf(listObj0) === protoList0, "property name list literal proto 0");
check(listObj0.__proto__ === secondComputedProto0, "property name list computed last wins 0");
check(listObj0.inheritedList === 20, "property name list inherited 0");
check(listObj0.value0 === 20 + 3, "property name list value 0");
var keyName0 = "__proto__";
var computedOnlyObj0 = { [keyName0]: firstComputedProto0, [keyName0]: secondComputedProto0 };
check(Object.getPrototypeOf(computedOnlyObj0) === Object.prototype, "computed only proto base 0");
check(computedOnlyObj0.__proto__ === secondComputedProto0, "computed only proto own 0");

var protoList1 = { inheritedList: 27 };
var firstComputedProto1 = { data: 27 + 1 };
var secondComputedProto1 = { data: 27 + 2 };
var listObj1 = { ["__proto__"]: firstComputedProto1, __proto__: protoList1, ["__proto__"]: secondComputedProto1, value1: 27 + 3 };
check(Object.getPrototypeOf(listObj1) === protoList1, "property name list literal proto 1");
check(listObj1.__proto__ === secondComputedProto1, "property name list computed last wins 1");
check(listObj1.inheritedList === 27, "property name list inherited 1");
check(listObj1.value1 === 27 + 3, "property name list value 1");
var keyName1 = "__proto__";
var computedOnlyObj1 = { [keyName1]: firstComputedProto1, [keyName1]: secondComputedProto1 };
check(Object.getPrototypeOf(computedOnlyObj1) === Object.prototype, "computed only proto base 1");
check(computedOnlyObj1.__proto__ === secondComputedProto1, "computed only proto own 1");

var protoList2 = { inheritedList: 34 };
var firstComputedProto2 = { data: 34 + 1 };
var secondComputedProto2 = { data: 34 + 2 };
var listObj2 = { ["__proto__"]: firstComputedProto2, __proto__: protoList2, ["__proto__"]: secondComputedProto2, value2: 34 + 3 };
check(Object.getPrototypeOf(listObj2) === protoList2, "property name list literal proto 2");
check(listObj2.__proto__ === secondComputedProto2, "property name list computed last wins 2");
check(listObj2.inheritedList === 34, "property name list inherited 2");
check(listObj2.value2 === 34 + 3, "property name list value 2");
var keyName2 = "__proto__";
var computedOnlyObj2 = { [keyName2]: firstComputedProto2, [keyName2]: secondComputedProto2 };
check(Object.getPrototypeOf(computedOnlyObj2) === Object.prototype, "computed only proto base 2");
check(computedOnlyObj2.__proto__ === secondComputedProto2, "computed only proto own 2");

var protoList3 = { inheritedList: 41 };
var firstComputedProto3 = { data: 41 + 1 };
var secondComputedProto3 = { data: 41 + 2 };
var listObj3 = { ["__proto__"]: firstComputedProto3, __proto__: protoList3, ["__proto__"]: secondComputedProto3, value3: 41 + 3 };
check(Object.getPrototypeOf(listObj3) === protoList3, "property name list literal proto 3");
check(listObj3.__proto__ === secondComputedProto3, "property name list computed last wins 3");
check(listObj3.inheritedList === 41, "property name list inherited 3");
check(listObj3.value3 === 41 + 3, "property name list value 3");
var keyName3 = "__proto__";
var computedOnlyObj3 = { [keyName3]: firstComputedProto3, [keyName3]: secondComputedProto3 };
check(Object.getPrototypeOf(computedOnlyObj3) === Object.prototype, "computed only proto base 3");
check(computedOnlyObj3.__proto__ === secondComputedProto3, "computed only proto own 3");

var protoList4 = { inheritedList: 48 };
var firstComputedProto4 = { data: 48 + 1 };
var secondComputedProto4 = { data: 48 + 2 };
var listObj4 = { ["__proto__"]: firstComputedProto4, __proto__: protoList4, ["__proto__"]: secondComputedProto4, value4: 48 + 3 };
check(Object.getPrototypeOf(listObj4) === protoList4, "property name list literal proto 4");
check(listObj4.__proto__ === secondComputedProto4, "property name list computed last wins 4");
check(listObj4.inheritedList === 48, "property name list inherited 4");
check(listObj4.value4 === 48 + 3, "property name list value 4");
var keyName4 = "__proto__";
var computedOnlyObj4 = { [keyName4]: firstComputedProto4, [keyName4]: secondComputedProto4 };
check(Object.getPrototypeOf(computedOnlyObj4) === Object.prototype, "computed only proto base 4");
check(computedOnlyObj4.__proto__ === secondComputedProto4, "computed only proto own 4");

var protoList5 = { inheritedList: 55 };
var firstComputedProto5 = { data: 55 + 1 };
var secondComputedProto5 = { data: 55 + 2 };
var listObj5 = { ["__proto__"]: firstComputedProto5, __proto__: protoList5, ["__proto__"]: secondComputedProto5, value5: 55 + 3 };
check(Object.getPrototypeOf(listObj5) === protoList5, "property name list literal proto 5");
check(listObj5.__proto__ === secondComputedProto5, "property name list computed last wins 5");
check(listObj5.inheritedList === 55, "property name list inherited 5");
check(listObj5.value5 === 55 + 3, "property name list value 5");
var keyName5 = "__proto__";
var computedOnlyObj5 = { [keyName5]: firstComputedProto5, [keyName5]: secondComputedProto5 };
check(Object.getPrototypeOf(computedOnlyObj5) === Object.prototype, "computed only proto base 5");
check(computedOnlyObj5.__proto__ === secondComputedProto5, "computed only proto own 5");

var protoList6 = { inheritedList: 62 };
var firstComputedProto6 = { data: 62 + 1 };
var secondComputedProto6 = { data: 62 + 2 };
var listObj6 = { ["__proto__"]: firstComputedProto6, __proto__: protoList6, ["__proto__"]: secondComputedProto6, value6: 62 + 3 };
check(Object.getPrototypeOf(listObj6) === protoList6, "property name list literal proto 6");
check(listObj6.__proto__ === secondComputedProto6, "property name list computed last wins 6");
check(listObj6.inheritedList === 62, "property name list inherited 6");
check(listObj6.value6 === 62 + 3, "property name list value 6");
var keyName6 = "__proto__";
var computedOnlyObj6 = { [keyName6]: firstComputedProto6, [keyName6]: secondComputedProto6 };
check(Object.getPrototypeOf(computedOnlyObj6) === Object.prototype, "computed only proto base 6");
check(computedOnlyObj6.__proto__ === secondComputedProto6, "computed only proto own 6");

var protoList7 = { inheritedList: 69 };
var firstComputedProto7 = { data: 69 + 1 };
var secondComputedProto7 = { data: 69 + 2 };
var listObj7 = { ["__proto__"]: firstComputedProto7, __proto__: protoList7, ["__proto__"]: secondComputedProto7, value7: 69 + 3 };
check(Object.getPrototypeOf(listObj7) === protoList7, "property name list literal proto 7");
check(listObj7.__proto__ === secondComputedProto7, "property name list computed last wins 7");
check(listObj7.inheritedList === 69, "property name list inherited 7");
check(listObj7.value7 === 69 + 3, "property name list value 7");
var keyName7 = "__proto__";
var computedOnlyObj7 = { [keyName7]: firstComputedProto7, [keyName7]: secondComputedProto7 };
check(Object.getPrototypeOf(computedOnlyObj7) === Object.prototype, "computed only proto base 7");
check(computedOnlyObj7.__proto__ === secondComputedProto7, "computed only proto own 7");

var protoList8 = { inheritedList: 76 };
var firstComputedProto8 = { data: 76 + 1 };
var secondComputedProto8 = { data: 76 + 2 };
var listObj8 = { ["__proto__"]: firstComputedProto8, __proto__: protoList8, ["__proto__"]: secondComputedProto8, value8: 76 + 3 };
check(Object.getPrototypeOf(listObj8) === protoList8, "property name list literal proto 8");
check(listObj8.__proto__ === secondComputedProto8, "property name list computed last wins 8");
check(listObj8.inheritedList === 76, "property name list inherited 8");
check(listObj8.value8 === 76 + 3, "property name list value 8");
var keyName8 = "__proto__";
var computedOnlyObj8 = { [keyName8]: firstComputedProto8, [keyName8]: secondComputedProto8 };
check(Object.getPrototypeOf(computedOnlyObj8) === Object.prototype, "computed only proto base 8");
check(computedOnlyObj8.__proto__ === secondComputedProto8, "computed only proto own 8");

var protoList9 = { inheritedList: 83 };
var firstComputedProto9 = { data: 83 + 1 };
var secondComputedProto9 = { data: 83 + 2 };
var listObj9 = { ["__proto__"]: firstComputedProto9, __proto__: protoList9, ["__proto__"]: secondComputedProto9, value9: 83 + 3 };
check(Object.getPrototypeOf(listObj9) === protoList9, "property name list literal proto 9");
check(listObj9.__proto__ === secondComputedProto9, "property name list computed last wins 9");
check(listObj9.inheritedList === 83, "property name list inherited 9");
check(listObj9.value9 === 83 + 3, "property name list value 9");
var keyName9 = "__proto__";
var computedOnlyObj9 = { [keyName9]: firstComputedProto9, [keyName9]: secondComputedProto9 };
check(Object.getPrototypeOf(computedOnlyObj9) === Object.prototype, "computed only proto base 9");
check(computedOnlyObj9.__proto__ === secondComputedProto9, "computed only proto own 9");

check(score > 0, "property name list score");
