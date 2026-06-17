// behavior: assignment-elements-create-index-properties
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
function readValue(value) { return value; }
var holder0 = { value: 21 };
var source0 = [22, 23];
var assignedElements0 = [holder0.value, source0[1], readValue(24)];
check(assignedElements0.length === 3, 'assigned length 0');
check((0 in assignedElements0) && (1 in assignedElements0) && (2 in assignedElements0), 'assigned properties 0');
check(assignedElements0[0] === 21, 'assigned holder value 0');
check(assignedElements0[1] === 23, 'assigned source value 0');
check(assignedElements0[2] === 24, 'assigned call value 0');
holder0.value = 121;
source0[1] = 221;
check(assignedElements0[0] === 21 && assignedElements0[1] === 23, 'assigned stores values 0');
var holder1 = { value: 22 };
var source1 = [23, 24];
var assignedElements1 = [holder1.value, source1[1], readValue(25)];
check(assignedElements1.length === 3, 'assigned length 1');
check((0 in assignedElements1) && (1 in assignedElements1) && (2 in assignedElements1), 'assigned properties 1');
check(assignedElements1[0] === 22, 'assigned holder value 1');
check(assignedElements1[1] === 24, 'assigned source value 1');
check(assignedElements1[2] === 25, 'assigned call value 1');
holder1.value = 122;
source1[1] = 222;
check(assignedElements1[0] === 22 && assignedElements1[1] === 24, 'assigned stores values 1');
var holder2 = { value: 23 };
var source2 = [24, 25];
var assignedElements2 = [holder2.value, source2[1], readValue(26)];
check(assignedElements2.length === 3, 'assigned length 2');
check((0 in assignedElements2) && (1 in assignedElements2) && (2 in assignedElements2), 'assigned properties 2');
check(assignedElements2[0] === 23, 'assigned holder value 2');
check(assignedElements2[1] === 25, 'assigned source value 2');
check(assignedElements2[2] === 26, 'assigned call value 2');
holder2.value = 123;
source2[1] = 223;
check(assignedElements2[0] === 23 && assignedElements2[1] === 25, 'assigned stores values 2');
var holder3 = { value: 24 };
var source3 = [25, 26];
var assignedElements3 = [holder3.value, source3[1], readValue(27)];
check(assignedElements3.length === 3, 'assigned length 3');
check((0 in assignedElements3) && (1 in assignedElements3) && (2 in assignedElements3), 'assigned properties 3');
check(assignedElements3[0] === 24, 'assigned holder value 3');
check(assignedElements3[1] === 26, 'assigned source value 3');
check(assignedElements3[2] === 27, 'assigned call value 3');
holder3.value = 124;
source3[1] = 224;
check(assignedElements3[0] === 24 && assignedElements3[1] === 26, 'assigned stores values 3');
var holder4 = { value: 25 };
var source4 = [26, 27];
var assignedElements4 = [holder4.value, source4[1], readValue(28)];
check(assignedElements4.length === 3, 'assigned length 4');
check((0 in assignedElements4) && (1 in assignedElements4) && (2 in assignedElements4), 'assigned properties 4');
check(assignedElements4[0] === 25, 'assigned holder value 4');
check(assignedElements4[1] === 27, 'assigned source value 4');
check(assignedElements4[2] === 28, 'assigned call value 4');
holder4.value = 125;
source4[1] = 225;
check(assignedElements4[0] === 25 && assignedElements4[1] === 27, 'assigned stores values 4');
var holder5 = { value: 26 };
var source5 = [27, 28];
var assignedElements5 = [holder5.value, source5[1], readValue(29)];
check(assignedElements5.length === 3, 'assigned length 5');
check((0 in assignedElements5) && (1 in assignedElements5) && (2 in assignedElements5), 'assigned properties 5');
check(assignedElements5[0] === 26, 'assigned holder value 5');
check(assignedElements5[1] === 28, 'assigned source value 5');
check(assignedElements5[2] === 29, 'assigned call value 5');
holder5.value = 126;
source5[1] = 226;
check(assignedElements5[0] === 26 && assignedElements5[1] === 28, 'assigned stores values 5');
var holder6 = { value: 27 };
var source6 = [28, 29];
var assignedElements6 = [holder6.value, source6[1], readValue(30)];
check(assignedElements6.length === 3, 'assigned length 6');
check((0 in assignedElements6) && (1 in assignedElements6) && (2 in assignedElements6), 'assigned properties 6');
check(assignedElements6[0] === 27, 'assigned holder value 6');
check(assignedElements6[1] === 29, 'assigned source value 6');
check(assignedElements6[2] === 30, 'assigned call value 6');
holder6.value = 127;
source6[1] = 227;
check(assignedElements6[0] === 27 && assignedElements6[1] === 29, 'assigned stores values 6');
var holder7 = { value: 28 };
var source7 = [29, 30];
var assignedElements7 = [holder7.value, source7[1], readValue(31)];
check(assignedElements7.length === 3, 'assigned length 7');
check((0 in assignedElements7) && (1 in assignedElements7) && (2 in assignedElements7), 'assigned properties 7');
check(assignedElements7[0] === 28, 'assigned holder value 7');
check(assignedElements7[1] === 30, 'assigned source value 7');
check(assignedElements7[2] === 31, 'assigned call value 7');
holder7.value = 128;
source7[1] = 228;
check(assignedElements7[0] === 28 && assignedElements7[1] === 30, 'assigned stores values 7');
var holder8 = { value: 29 };
var source8 = [30, 31];
var assignedElements8 = [holder8.value, source8[1], readValue(32)];
check(assignedElements8.length === 3, 'assigned length 8');
check((0 in assignedElements8) && (1 in assignedElements8) && (2 in assignedElements8), 'assigned properties 8');
check(assignedElements8[0] === 29, 'assigned holder value 8');
check(assignedElements8[1] === 31, 'assigned source value 8');
check(assignedElements8[2] === 32, 'assigned call value 8');
holder8.value = 129;
source8[1] = 229;
check(assignedElements8[0] === 29 && assignedElements8[1] === 31, 'assigned stores values 8');
console.log('assignment-elements-create-index-properties standard ' + score);
