// behavior: null-literal-evaluates-null
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
var missingValue = undefined;
function returnNull(value) { if (value) { return null; } return null; }
var nullValue0 = null;
check(nullValue0 === null, 'null strict 0');
check(!(nullValue0 !== null), 'null strict inverse 0');
check(nullValue0 == missingValue, 'null loose undefined 0');
check(!(nullValue0 === missingValue), 'null not undefined 0');
check(typeof nullValue0 === 'object', 'null typeof 0');
check((true ? null : 0) === nullValue0, 'null ternary true 0');
check((false ? 0 : null) === nullValue0, 'null ternary false 0');
check(returnNull(true) === null, 'null return 0');
var nullValue1 = null;
check(nullValue1 === null, 'null strict 1');
check(!(nullValue1 !== null), 'null strict inverse 1');
check(nullValue1 == missingValue, 'null loose undefined 1');
check(!(nullValue1 === missingValue), 'null not undefined 1');
check(typeof nullValue1 === 'object', 'null typeof 1');
check((true ? null : 1) === nullValue1, 'null ternary true 1');
check((false ? 1 : null) === nullValue1, 'null ternary false 1');
check(returnNull(false) === null, 'null return 1');
var nullValue2 = null;
check(nullValue2 === null, 'null strict 2');
check(!(nullValue2 !== null), 'null strict inverse 2');
check(nullValue2 == missingValue, 'null loose undefined 2');
check(!(nullValue2 === missingValue), 'null not undefined 2');
check(typeof nullValue2 === 'object', 'null typeof 2');
check((true ? null : 2) === nullValue2, 'null ternary true 2');
check((false ? 2 : null) === nullValue2, 'null ternary false 2');
check(returnNull(true) === null, 'null return 2');
var nullValue3 = null;
check(nullValue3 === null, 'null strict 3');
check(!(nullValue3 !== null), 'null strict inverse 3');
check(nullValue3 == missingValue, 'null loose undefined 3');
check(!(nullValue3 === missingValue), 'null not undefined 3');
check(typeof nullValue3 === 'object', 'null typeof 3');
check((true ? null : 3) === nullValue3, 'null ternary true 3');
check((false ? 3 : null) === nullValue3, 'null ternary false 3');
check(returnNull(false) === null, 'null return 3');
var nullValue4 = null;
check(nullValue4 === null, 'null strict 4');
check(!(nullValue4 !== null), 'null strict inverse 4');
check(nullValue4 == missingValue, 'null loose undefined 4');
check(!(nullValue4 === missingValue), 'null not undefined 4');
check(typeof nullValue4 === 'object', 'null typeof 4');
check((true ? null : 4) === nullValue4, 'null ternary true 4');
check((false ? 4 : null) === nullValue4, 'null ternary false 4');
check(returnNull(true) === null, 'null return 4');
var nullValue5 = null;
check(nullValue5 === null, 'null strict 5');
check(!(nullValue5 !== null), 'null strict inverse 5');
check(nullValue5 == missingValue, 'null loose undefined 5');
check(!(nullValue5 === missingValue), 'null not undefined 5');
check(typeof nullValue5 === 'object', 'null typeof 5');
check((true ? null : 5) === nullValue5, 'null ternary true 5');
check((false ? 5 : null) === nullValue5, 'null ternary false 5');
check(returnNull(false) === null, 'null return 5');
var nullValue6 = null;
check(nullValue6 === null, 'null strict 6');
check(!(nullValue6 !== null), 'null strict inverse 6');
check(nullValue6 == missingValue, 'null loose undefined 6');
check(!(nullValue6 === missingValue), 'null not undefined 6');
check(typeof nullValue6 === 'object', 'null typeof 6');
check((true ? null : 6) === nullValue6, 'null ternary true 6');
check((false ? 6 : null) === nullValue6, 'null ternary false 6');
check(returnNull(true) === null, 'null return 6');
var nullValue7 = null;
check(nullValue7 === null, 'null strict 7');
check(!(nullValue7 !== null), 'null strict inverse 7');
check(nullValue7 == missingValue, 'null loose undefined 7');
check(!(nullValue7 === missingValue), 'null not undefined 7');
check(typeof nullValue7 === 'object', 'null typeof 7');
check((true ? null : 7) === nullValue7, 'null ternary true 7');
check((false ? 7 : null) === nullValue7, 'null ternary false 7');
check(returnNull(false) === null, 'null return 7');
var nullValue8 = null;
check(nullValue8 === null, 'null strict 8');
check(!(nullValue8 !== null), 'null strict inverse 8');
check(nullValue8 == missingValue, 'null loose undefined 8');
check(!(nullValue8 === missingValue), 'null not undefined 8');
check(typeof nullValue8 === 'object', 'null typeof 8');
check((true ? null : 8) === nullValue8, 'null ternary true 8');
check((false ? 8 : null) === nullValue8, 'null ternary false 8');
check(returnNull(true) === null, 'null return 8');
var nullValue9 = null;
check(nullValue9 === null, 'null strict 9');
check(!(nullValue9 !== null), 'null strict inverse 9');
check(nullValue9 == missingValue, 'null loose undefined 9');
check(!(nullValue9 === missingValue), 'null not undefined 9');
check(typeof nullValue9 === 'object', 'null typeof 9');
check((true ? null : 9) === nullValue9, 'null ternary true 9');
check((false ? 9 : null) === nullValue9, 'null ternary false 9');
check(returnNull(false) === null, 'null return 9');
var nullValue10 = null;
check(nullValue10 === null, 'null strict 10');
check(!(nullValue10 !== null), 'null strict inverse 10');
check(nullValue10 == missingValue, 'null loose undefined 10');
check(!(nullValue10 === missingValue), 'null not undefined 10');
check(typeof nullValue10 === 'object', 'null typeof 10');
check((true ? null : 10) === nullValue10, 'null ternary true 10');
check((false ? 10 : null) === nullValue10, 'null ternary false 10');
check(returnNull(true) === null, 'null return 10');
console.log('null-literal-evaluates-null standard ' + score);
