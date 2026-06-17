// behavior: elements-evaluate-left-to-right
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
function mark(value) { orderLog[orderLog.length] = value; return value; }
function orderStart() { return orderLog.length; }
var startOrder0 = orderStart();
var orderedArray0 = [mark(1), , mark(2), ...[mark(3), mark(4)], mark(5)];
check(orderedArray0.length === 6, 'ordered length 0');
check(orderedArray0[0] === 1 && !(1 in orderedArray0) && orderedArray0[2] === 2, 'ordered prefix 0');
check(orderedArray0[3] === 3 && orderedArray0[4] === 4 && orderedArray0[5] === 5, 'ordered suffix 0');
check(orderLog[startOrder0] === 1, 'order first 0');
check(orderLog[startOrder0 + 1] === 2, 'order second 0');
check(orderLog[startOrder0 + 2] === 3, 'order spread first 0');
check(orderLog[startOrder0 + 3] === 4, 'order spread second 0');
check(orderLog[startOrder0 + 4] === 5, 'order last 0');
var startOrder1 = orderStart();
var orderedArray1 = [mark(11), , mark(12), ...[mark(13), mark(14)], mark(15)];
check(orderedArray1.length === 6, 'ordered length 1');
check(orderedArray1[0] === 11 && !(1 in orderedArray1) && orderedArray1[2] === 12, 'ordered prefix 1');
check(orderedArray1[3] === 13 && orderedArray1[4] === 14 && orderedArray1[5] === 15, 'ordered suffix 1');
check(orderLog[startOrder1] === 11, 'order first 1');
check(orderLog[startOrder1 + 1] === 12, 'order second 1');
check(orderLog[startOrder1 + 2] === 13, 'order spread first 1');
check(orderLog[startOrder1 + 3] === 14, 'order spread second 1');
check(orderLog[startOrder1 + 4] === 15, 'order last 1');
var startOrder2 = orderStart();
var orderedArray2 = [mark(21), , mark(22), ...[mark(23), mark(24)], mark(25)];
check(orderedArray2.length === 6, 'ordered length 2');
check(orderedArray2[0] === 21 && !(1 in orderedArray2) && orderedArray2[2] === 22, 'ordered prefix 2');
check(orderedArray2[3] === 23 && orderedArray2[4] === 24 && orderedArray2[5] === 25, 'ordered suffix 2');
check(orderLog[startOrder2] === 21, 'order first 2');
check(orderLog[startOrder2 + 1] === 22, 'order second 2');
check(orderLog[startOrder2 + 2] === 23, 'order spread first 2');
check(orderLog[startOrder2 + 3] === 24, 'order spread second 2');
check(orderLog[startOrder2 + 4] === 25, 'order last 2');
var startOrder3 = orderStart();
var orderedArray3 = [mark(31), , mark(32), ...[mark(33), mark(34)], mark(35)];
check(orderedArray3.length === 6, 'ordered length 3');
check(orderedArray3[0] === 31 && !(1 in orderedArray3) && orderedArray3[2] === 32, 'ordered prefix 3');
check(orderedArray3[3] === 33 && orderedArray3[4] === 34 && orderedArray3[5] === 35, 'ordered suffix 3');
check(orderLog[startOrder3] === 31, 'order first 3');
check(orderLog[startOrder3 + 1] === 32, 'order second 3');
check(orderLog[startOrder3 + 2] === 33, 'order spread first 3');
check(orderLog[startOrder3 + 3] === 34, 'order spread second 3');
check(orderLog[startOrder3 + 4] === 35, 'order last 3');
var startOrder4 = orderStart();
var orderedArray4 = [mark(41), , mark(42), ...[mark(43), mark(44)], mark(45)];
check(orderedArray4.length === 6, 'ordered length 4');
check(orderedArray4[0] === 41 && !(1 in orderedArray4) && orderedArray4[2] === 42, 'ordered prefix 4');
check(orderedArray4[3] === 43 && orderedArray4[4] === 44 && orderedArray4[5] === 45, 'ordered suffix 4');
check(orderLog[startOrder4] === 41, 'order first 4');
check(orderLog[startOrder4 + 1] === 42, 'order second 4');
check(orderLog[startOrder4 + 2] === 43, 'order spread first 4');
check(orderLog[startOrder4 + 3] === 44, 'order spread second 4');
check(orderLog[startOrder4 + 4] === 45, 'order last 4');
var startOrder5 = orderStart();
var orderedArray5 = [mark(51), , mark(52), ...[mark(53), mark(54)], mark(55)];
check(orderedArray5.length === 6, 'ordered length 5');
check(orderedArray5[0] === 51 && !(1 in orderedArray5) && orderedArray5[2] === 52, 'ordered prefix 5');
check(orderedArray5[3] === 53 && orderedArray5[4] === 54 && orderedArray5[5] === 55, 'ordered suffix 5');
check(orderLog[startOrder5] === 51, 'order first 5');
check(orderLog[startOrder5 + 1] === 52, 'order second 5');
check(orderLog[startOrder5 + 2] === 53, 'order spread first 5');
check(orderLog[startOrder5 + 3] === 54, 'order spread second 5');
check(orderLog[startOrder5 + 4] === 55, 'order last 5');
var startOrder6 = orderStart();
var orderedArray6 = [mark(61), , mark(62), ...[mark(63), mark(64)], mark(65)];
check(orderedArray6.length === 6, 'ordered length 6');
check(orderedArray6[0] === 61 && !(1 in orderedArray6) && orderedArray6[2] === 62, 'ordered prefix 6');
check(orderedArray6[3] === 63 && orderedArray6[4] === 64 && orderedArray6[5] === 65, 'ordered suffix 6');
check(orderLog[startOrder6] === 61, 'order first 6');
check(orderLog[startOrder6 + 1] === 62, 'order second 6');
check(orderLog[startOrder6 + 2] === 63, 'order spread first 6');
check(orderLog[startOrder6 + 3] === 64, 'order spread second 6');
check(orderLog[startOrder6 + 4] === 65, 'order last 6');
var startOrder7 = orderStart();
var orderedArray7 = [mark(71), , mark(72), ...[mark(73), mark(74)], mark(75)];
check(orderedArray7.length === 6, 'ordered length 7');
check(orderedArray7[0] === 71 && !(1 in orderedArray7) && orderedArray7[2] === 72, 'ordered prefix 7');
check(orderedArray7[3] === 73 && orderedArray7[4] === 74 && orderedArray7[5] === 75, 'ordered suffix 7');
check(orderLog[startOrder7] === 71, 'order first 7');
check(orderLog[startOrder7 + 1] === 72, 'order second 7');
check(orderLog[startOrder7 + 2] === 73, 'order spread first 7');
check(orderLog[startOrder7 + 3] === 74, 'order spread second 7');
check(orderLog[startOrder7 + 4] === 75, 'order last 7');
var startOrder8 = orderStart();
var orderedArray8 = [mark(81), , mark(82), ...[mark(83), mark(84)], mark(85)];
check(orderedArray8.length === 6, 'ordered length 8');
check(orderedArray8[0] === 81 && !(1 in orderedArray8) && orderedArray8[2] === 82, 'ordered prefix 8');
check(orderedArray8[3] === 83 && orderedArray8[4] === 84 && orderedArray8[5] === 85, 'ordered suffix 8');
check(orderLog[startOrder8] === 81, 'order first 8');
check(orderLog[startOrder8 + 1] === 82, 'order second 8');
check(orderLog[startOrder8 + 2] === 83, 'order spread first 8');
check(orderLog[startOrder8 + 3] === 84, 'order spread second 8');
check(orderLog[startOrder8 + 4] === 85, 'order last 8');
var startOrder9 = orderStart();
var orderedArray9 = [mark(91), , mark(92), ...[mark(93), mark(94)], mark(95)];
check(orderedArray9.length === 6, 'ordered length 9');
check(orderedArray9[0] === 91 && !(1 in orderedArray9) && orderedArray9[2] === 92, 'ordered prefix 9');
check(orderedArray9[3] === 93 && orderedArray9[4] === 94 && orderedArray9[5] === 95, 'ordered suffix 9');
check(orderLog[startOrder9] === 91, 'order first 9');
check(orderLog[startOrder9 + 1] === 92, 'order second 9');
check(orderLog[startOrder9 + 2] === 93, 'order spread first 9');
check(orderLog[startOrder9 + 3] === 94, 'order spread second 9');
check(orderLog[startOrder9 + 4] === 95, 'order last 9');
console.log('elements-evaluate-left-to-right standard ' + score);
