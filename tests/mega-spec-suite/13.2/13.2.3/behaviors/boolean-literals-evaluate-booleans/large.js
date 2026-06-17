// behavior: boolean-literals-evaluate-booleans
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
function chooseBoolean(left, right, takeLeft) { if (takeLeft) { return left; } return right; }
var truthyLiteral0 = true;
var falsyLiteral0 = false;
check(truthyLiteral0 === true, 'true strict 0');
check(falsyLiteral0 === false, 'false strict 0');
check(truthyLiteral0 !== falsyLiteral0, 'boolean distinct 0');
check(typeof truthyLiteral0 === 'boolean', 'true typeof 0');
check(typeof falsyLiteral0 === 'boolean', 'false typeof 0');
check((truthyLiteral0 && !falsyLiteral0) === true, 'boolean and not 0');
check((falsyLiteral0 || truthyLiteral0) === true, 'boolean or 0');
check((truthyLiteral0 ? 1 : 2) === 1, 'true conditional 0');
check((falsyLiteral0 ? 1 : 2) === 2, 'false conditional 0');
if (truthyLiteral0) { score = score + 1; } else { throw 'true branch 0'; }
if (falsyLiteral0) { throw 'false branch 0'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral0, falsyLiteral0, true) === true, 'choose true 0');
check(chooseBoolean(truthyLiteral0, falsyLiteral0, false) === false, 'choose false 0');
var truthyLiteral1 = true;
var falsyLiteral1 = false;
check(truthyLiteral1 === true, 'true strict 1');
check(falsyLiteral1 === false, 'false strict 1');
check(truthyLiteral1 !== falsyLiteral1, 'boolean distinct 1');
check(typeof truthyLiteral1 === 'boolean', 'true typeof 1');
check(typeof falsyLiteral1 === 'boolean', 'false typeof 1');
check((truthyLiteral1 && !falsyLiteral1) === true, 'boolean and not 1');
check((falsyLiteral1 || truthyLiteral1) === true, 'boolean or 1');
check((truthyLiteral1 ? 2 : 3) === 2, 'true conditional 1');
check((falsyLiteral1 ? 2 : 3) === 3, 'false conditional 1');
if (truthyLiteral1) { score = score + 1; } else { throw 'true branch 1'; }
if (falsyLiteral1) { throw 'false branch 1'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral1, falsyLiteral1, true) === true, 'choose true 1');
check(chooseBoolean(truthyLiteral1, falsyLiteral1, false) === false, 'choose false 1');
var truthyLiteral2 = true;
var falsyLiteral2 = false;
check(truthyLiteral2 === true, 'true strict 2');
check(falsyLiteral2 === false, 'false strict 2');
check(truthyLiteral2 !== falsyLiteral2, 'boolean distinct 2');
check(typeof truthyLiteral2 === 'boolean', 'true typeof 2');
check(typeof falsyLiteral2 === 'boolean', 'false typeof 2');
check((truthyLiteral2 && !falsyLiteral2) === true, 'boolean and not 2');
check((falsyLiteral2 || truthyLiteral2) === true, 'boolean or 2');
check((truthyLiteral2 ? 3 : 4) === 3, 'true conditional 2');
check((falsyLiteral2 ? 3 : 4) === 4, 'false conditional 2');
if (truthyLiteral2) { score = score + 1; } else { throw 'true branch 2'; }
if (falsyLiteral2) { throw 'false branch 2'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral2, falsyLiteral2, true) === true, 'choose true 2');
check(chooseBoolean(truthyLiteral2, falsyLiteral2, false) === false, 'choose false 2');
var truthyLiteral3 = true;
var falsyLiteral3 = false;
check(truthyLiteral3 === true, 'true strict 3');
check(falsyLiteral3 === false, 'false strict 3');
check(truthyLiteral3 !== falsyLiteral3, 'boolean distinct 3');
check(typeof truthyLiteral3 === 'boolean', 'true typeof 3');
check(typeof falsyLiteral3 === 'boolean', 'false typeof 3');
check((truthyLiteral3 && !falsyLiteral3) === true, 'boolean and not 3');
check((falsyLiteral3 || truthyLiteral3) === true, 'boolean or 3');
check((truthyLiteral3 ? 4 : 5) === 4, 'true conditional 3');
check((falsyLiteral3 ? 4 : 5) === 5, 'false conditional 3');
if (truthyLiteral3) { score = score + 1; } else { throw 'true branch 3'; }
if (falsyLiteral3) { throw 'false branch 3'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral3, falsyLiteral3, true) === true, 'choose true 3');
check(chooseBoolean(truthyLiteral3, falsyLiteral3, false) === false, 'choose false 3');
var truthyLiteral4 = true;
var falsyLiteral4 = false;
check(truthyLiteral4 === true, 'true strict 4');
check(falsyLiteral4 === false, 'false strict 4');
check(truthyLiteral4 !== falsyLiteral4, 'boolean distinct 4');
check(typeof truthyLiteral4 === 'boolean', 'true typeof 4');
check(typeof falsyLiteral4 === 'boolean', 'false typeof 4');
check((truthyLiteral4 && !falsyLiteral4) === true, 'boolean and not 4');
check((falsyLiteral4 || truthyLiteral4) === true, 'boolean or 4');
check((truthyLiteral4 ? 5 : 6) === 5, 'true conditional 4');
check((falsyLiteral4 ? 5 : 6) === 6, 'false conditional 4');
if (truthyLiteral4) { score = score + 1; } else { throw 'true branch 4'; }
if (falsyLiteral4) { throw 'false branch 4'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral4, falsyLiteral4, true) === true, 'choose true 4');
check(chooseBoolean(truthyLiteral4, falsyLiteral4, false) === false, 'choose false 4');
var truthyLiteral5 = true;
var falsyLiteral5 = false;
check(truthyLiteral5 === true, 'true strict 5');
check(falsyLiteral5 === false, 'false strict 5');
check(truthyLiteral5 !== falsyLiteral5, 'boolean distinct 5');
check(typeof truthyLiteral5 === 'boolean', 'true typeof 5');
check(typeof falsyLiteral5 === 'boolean', 'false typeof 5');
check((truthyLiteral5 && !falsyLiteral5) === true, 'boolean and not 5');
check((falsyLiteral5 || truthyLiteral5) === true, 'boolean or 5');
check((truthyLiteral5 ? 6 : 7) === 6, 'true conditional 5');
check((falsyLiteral5 ? 6 : 7) === 7, 'false conditional 5');
if (truthyLiteral5) { score = score + 1; } else { throw 'true branch 5'; }
if (falsyLiteral5) { throw 'false branch 5'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral5, falsyLiteral5, true) === true, 'choose true 5');
check(chooseBoolean(truthyLiteral5, falsyLiteral5, false) === false, 'choose false 5');
var truthyLiteral6 = true;
var falsyLiteral6 = false;
check(truthyLiteral6 === true, 'true strict 6');
check(falsyLiteral6 === false, 'false strict 6');
check(truthyLiteral6 !== falsyLiteral6, 'boolean distinct 6');
check(typeof truthyLiteral6 === 'boolean', 'true typeof 6');
check(typeof falsyLiteral6 === 'boolean', 'false typeof 6');
check((truthyLiteral6 && !falsyLiteral6) === true, 'boolean and not 6');
check((falsyLiteral6 || truthyLiteral6) === true, 'boolean or 6');
check((truthyLiteral6 ? 7 : 8) === 7, 'true conditional 6');
check((falsyLiteral6 ? 7 : 8) === 8, 'false conditional 6');
if (truthyLiteral6) { score = score + 1; } else { throw 'true branch 6'; }
if (falsyLiteral6) { throw 'false branch 6'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral6, falsyLiteral6, true) === true, 'choose true 6');
check(chooseBoolean(truthyLiteral6, falsyLiteral6, false) === false, 'choose false 6');
var truthyLiteral7 = true;
var falsyLiteral7 = false;
check(truthyLiteral7 === true, 'true strict 7');
check(falsyLiteral7 === false, 'false strict 7');
check(truthyLiteral7 !== falsyLiteral7, 'boolean distinct 7');
check(typeof truthyLiteral7 === 'boolean', 'true typeof 7');
check(typeof falsyLiteral7 === 'boolean', 'false typeof 7');
check((truthyLiteral7 && !falsyLiteral7) === true, 'boolean and not 7');
check((falsyLiteral7 || truthyLiteral7) === true, 'boolean or 7');
check((truthyLiteral7 ? 8 : 9) === 8, 'true conditional 7');
check((falsyLiteral7 ? 8 : 9) === 9, 'false conditional 7');
if (truthyLiteral7) { score = score + 1; } else { throw 'true branch 7'; }
if (falsyLiteral7) { throw 'false branch 7'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral7, falsyLiteral7, true) === true, 'choose true 7');
check(chooseBoolean(truthyLiteral7, falsyLiteral7, false) === false, 'choose false 7');
var truthyLiteral8 = true;
var falsyLiteral8 = false;
check(truthyLiteral8 === true, 'true strict 8');
check(falsyLiteral8 === false, 'false strict 8');
check(truthyLiteral8 !== falsyLiteral8, 'boolean distinct 8');
check(typeof truthyLiteral8 === 'boolean', 'true typeof 8');
check(typeof falsyLiteral8 === 'boolean', 'false typeof 8');
check((truthyLiteral8 && !falsyLiteral8) === true, 'boolean and not 8');
check((falsyLiteral8 || truthyLiteral8) === true, 'boolean or 8');
check((truthyLiteral8 ? 9 : 10) === 9, 'true conditional 8');
check((falsyLiteral8 ? 9 : 10) === 10, 'false conditional 8');
if (truthyLiteral8) { score = score + 1; } else { throw 'true branch 8'; }
if (falsyLiteral8) { throw 'false branch 8'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral8, falsyLiteral8, true) === true, 'choose true 8');
check(chooseBoolean(truthyLiteral8, falsyLiteral8, false) === false, 'choose false 8');
var truthyLiteral9 = true;
var falsyLiteral9 = false;
check(truthyLiteral9 === true, 'true strict 9');
check(falsyLiteral9 === false, 'false strict 9');
check(truthyLiteral9 !== falsyLiteral9, 'boolean distinct 9');
check(typeof truthyLiteral9 === 'boolean', 'true typeof 9');
check(typeof falsyLiteral9 === 'boolean', 'false typeof 9');
check((truthyLiteral9 && !falsyLiteral9) === true, 'boolean and not 9');
check((falsyLiteral9 || truthyLiteral9) === true, 'boolean or 9');
check((truthyLiteral9 ? 10 : 11) === 10, 'true conditional 9');
check((falsyLiteral9 ? 10 : 11) === 11, 'false conditional 9');
if (truthyLiteral9) { score = score + 1; } else { throw 'true branch 9'; }
if (falsyLiteral9) { throw 'false branch 9'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral9, falsyLiteral9, true) === true, 'choose true 9');
check(chooseBoolean(truthyLiteral9, falsyLiteral9, false) === false, 'choose false 9');
var truthyLiteral10 = true;
var falsyLiteral10 = false;
check(truthyLiteral10 === true, 'true strict 10');
check(falsyLiteral10 === false, 'false strict 10');
check(truthyLiteral10 !== falsyLiteral10, 'boolean distinct 10');
check(typeof truthyLiteral10 === 'boolean', 'true typeof 10');
check(typeof falsyLiteral10 === 'boolean', 'false typeof 10');
check((truthyLiteral10 && !falsyLiteral10) === true, 'boolean and not 10');
check((falsyLiteral10 || truthyLiteral10) === true, 'boolean or 10');
check((truthyLiteral10 ? 11 : 12) === 11, 'true conditional 10');
check((falsyLiteral10 ? 11 : 12) === 12, 'false conditional 10');
if (truthyLiteral10) { score = score + 1; } else { throw 'true branch 10'; }
if (falsyLiteral10) { throw 'false branch 10'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral10, falsyLiteral10, true) === true, 'choose true 10');
check(chooseBoolean(truthyLiteral10, falsyLiteral10, false) === false, 'choose false 10');
var truthyLiteral11 = true;
var falsyLiteral11 = false;
check(truthyLiteral11 === true, 'true strict 11');
check(falsyLiteral11 === false, 'false strict 11');
check(truthyLiteral11 !== falsyLiteral11, 'boolean distinct 11');
check(typeof truthyLiteral11 === 'boolean', 'true typeof 11');
check(typeof falsyLiteral11 === 'boolean', 'false typeof 11');
check((truthyLiteral11 && !falsyLiteral11) === true, 'boolean and not 11');
check((falsyLiteral11 || truthyLiteral11) === true, 'boolean or 11');
check((truthyLiteral11 ? 12 : 13) === 12, 'true conditional 11');
check((falsyLiteral11 ? 12 : 13) === 13, 'false conditional 11');
if (truthyLiteral11) { score = score + 1; } else { throw 'true branch 11'; }
if (falsyLiteral11) { throw 'false branch 11'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral11, falsyLiteral11, true) === true, 'choose true 11');
check(chooseBoolean(truthyLiteral11, falsyLiteral11, false) === false, 'choose false 11');
var truthyLiteral12 = true;
var falsyLiteral12 = false;
check(truthyLiteral12 === true, 'true strict 12');
check(falsyLiteral12 === false, 'false strict 12');
check(truthyLiteral12 !== falsyLiteral12, 'boolean distinct 12');
check(typeof truthyLiteral12 === 'boolean', 'true typeof 12');
check(typeof falsyLiteral12 === 'boolean', 'false typeof 12');
check((truthyLiteral12 && !falsyLiteral12) === true, 'boolean and not 12');
check((falsyLiteral12 || truthyLiteral12) === true, 'boolean or 12');
check((truthyLiteral12 ? 13 : 14) === 13, 'true conditional 12');
check((falsyLiteral12 ? 13 : 14) === 14, 'false conditional 12');
if (truthyLiteral12) { score = score + 1; } else { throw 'true branch 12'; }
if (falsyLiteral12) { throw 'false branch 12'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral12, falsyLiteral12, true) === true, 'choose true 12');
check(chooseBoolean(truthyLiteral12, falsyLiteral12, false) === false, 'choose false 12');
var truthyLiteral13 = true;
var falsyLiteral13 = false;
check(truthyLiteral13 === true, 'true strict 13');
check(falsyLiteral13 === false, 'false strict 13');
check(truthyLiteral13 !== falsyLiteral13, 'boolean distinct 13');
check(typeof truthyLiteral13 === 'boolean', 'true typeof 13');
check(typeof falsyLiteral13 === 'boolean', 'false typeof 13');
check((truthyLiteral13 && !falsyLiteral13) === true, 'boolean and not 13');
check((falsyLiteral13 || truthyLiteral13) === true, 'boolean or 13');
check((truthyLiteral13 ? 14 : 15) === 14, 'true conditional 13');
check((falsyLiteral13 ? 14 : 15) === 15, 'false conditional 13');
if (truthyLiteral13) { score = score + 1; } else { throw 'true branch 13'; }
if (falsyLiteral13) { throw 'false branch 13'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral13, falsyLiteral13, true) === true, 'choose true 13');
check(chooseBoolean(truthyLiteral13, falsyLiteral13, false) === false, 'choose false 13');
var truthyLiteral14 = true;
var falsyLiteral14 = false;
check(truthyLiteral14 === true, 'true strict 14');
check(falsyLiteral14 === false, 'false strict 14');
check(truthyLiteral14 !== falsyLiteral14, 'boolean distinct 14');
check(typeof truthyLiteral14 === 'boolean', 'true typeof 14');
check(typeof falsyLiteral14 === 'boolean', 'false typeof 14');
check((truthyLiteral14 && !falsyLiteral14) === true, 'boolean and not 14');
check((falsyLiteral14 || truthyLiteral14) === true, 'boolean or 14');
check((truthyLiteral14 ? 15 : 16) === 15, 'true conditional 14');
check((falsyLiteral14 ? 15 : 16) === 16, 'false conditional 14');
if (truthyLiteral14) { score = score + 1; } else { throw 'true branch 14'; }
if (falsyLiteral14) { throw 'false branch 14'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral14, falsyLiteral14, true) === true, 'choose true 14');
check(chooseBoolean(truthyLiteral14, falsyLiteral14, false) === false, 'choose false 14');
var truthyLiteral15 = true;
var falsyLiteral15 = false;
check(truthyLiteral15 === true, 'true strict 15');
check(falsyLiteral15 === false, 'false strict 15');
check(truthyLiteral15 !== falsyLiteral15, 'boolean distinct 15');
check(typeof truthyLiteral15 === 'boolean', 'true typeof 15');
check(typeof falsyLiteral15 === 'boolean', 'false typeof 15');
check((truthyLiteral15 && !falsyLiteral15) === true, 'boolean and not 15');
check((falsyLiteral15 || truthyLiteral15) === true, 'boolean or 15');
check((truthyLiteral15 ? 16 : 17) === 16, 'true conditional 15');
check((falsyLiteral15 ? 16 : 17) === 17, 'false conditional 15');
if (truthyLiteral15) { score = score + 1; } else { throw 'true branch 15'; }
if (falsyLiteral15) { throw 'false branch 15'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral15, falsyLiteral15, true) === true, 'choose true 15');
check(chooseBoolean(truthyLiteral15, falsyLiteral15, false) === false, 'choose false 15');
var truthyLiteral16 = true;
var falsyLiteral16 = false;
check(truthyLiteral16 === true, 'true strict 16');
check(falsyLiteral16 === false, 'false strict 16');
check(truthyLiteral16 !== falsyLiteral16, 'boolean distinct 16');
check(typeof truthyLiteral16 === 'boolean', 'true typeof 16');
check(typeof falsyLiteral16 === 'boolean', 'false typeof 16');
check((truthyLiteral16 && !falsyLiteral16) === true, 'boolean and not 16');
check((falsyLiteral16 || truthyLiteral16) === true, 'boolean or 16');
check((truthyLiteral16 ? 17 : 18) === 17, 'true conditional 16');
check((falsyLiteral16 ? 17 : 18) === 18, 'false conditional 16');
if (truthyLiteral16) { score = score + 1; } else { throw 'true branch 16'; }
if (falsyLiteral16) { throw 'false branch 16'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral16, falsyLiteral16, true) === true, 'choose true 16');
check(chooseBoolean(truthyLiteral16, falsyLiteral16, false) === false, 'choose false 16');
var truthyLiteral17 = true;
var falsyLiteral17 = false;
check(truthyLiteral17 === true, 'true strict 17');
check(falsyLiteral17 === false, 'false strict 17');
check(truthyLiteral17 !== falsyLiteral17, 'boolean distinct 17');
check(typeof truthyLiteral17 === 'boolean', 'true typeof 17');
check(typeof falsyLiteral17 === 'boolean', 'false typeof 17');
check((truthyLiteral17 && !falsyLiteral17) === true, 'boolean and not 17');
check((falsyLiteral17 || truthyLiteral17) === true, 'boolean or 17');
check((truthyLiteral17 ? 18 : 19) === 18, 'true conditional 17');
check((falsyLiteral17 ? 18 : 19) === 19, 'false conditional 17');
if (truthyLiteral17) { score = score + 1; } else { throw 'true branch 17'; }
if (falsyLiteral17) { throw 'false branch 17'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral17, falsyLiteral17, true) === true, 'choose true 17');
check(chooseBoolean(truthyLiteral17, falsyLiteral17, false) === false, 'choose false 17');
var truthyLiteral18 = true;
var falsyLiteral18 = false;
check(truthyLiteral18 === true, 'true strict 18');
check(falsyLiteral18 === false, 'false strict 18');
check(truthyLiteral18 !== falsyLiteral18, 'boolean distinct 18');
check(typeof truthyLiteral18 === 'boolean', 'true typeof 18');
check(typeof falsyLiteral18 === 'boolean', 'false typeof 18');
check((truthyLiteral18 && !falsyLiteral18) === true, 'boolean and not 18');
check((falsyLiteral18 || truthyLiteral18) === true, 'boolean or 18');
check((truthyLiteral18 ? 19 : 20) === 19, 'true conditional 18');
check((falsyLiteral18 ? 19 : 20) === 20, 'false conditional 18');
if (truthyLiteral18) { score = score + 1; } else { throw 'true branch 18'; }
if (falsyLiteral18) { throw 'false branch 18'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral18, falsyLiteral18, true) === true, 'choose true 18');
check(chooseBoolean(truthyLiteral18, falsyLiteral18, false) === false, 'choose false 18');
var truthyLiteral19 = true;
var falsyLiteral19 = false;
check(truthyLiteral19 === true, 'true strict 19');
check(falsyLiteral19 === false, 'false strict 19');
check(truthyLiteral19 !== falsyLiteral19, 'boolean distinct 19');
check(typeof truthyLiteral19 === 'boolean', 'true typeof 19');
check(typeof falsyLiteral19 === 'boolean', 'false typeof 19');
check((truthyLiteral19 && !falsyLiteral19) === true, 'boolean and not 19');
check((falsyLiteral19 || truthyLiteral19) === true, 'boolean or 19');
check((truthyLiteral19 ? 20 : 21) === 20, 'true conditional 19');
check((falsyLiteral19 ? 20 : 21) === 21, 'false conditional 19');
if (truthyLiteral19) { score = score + 1; } else { throw 'true branch 19'; }
if (falsyLiteral19) { throw 'false branch 19'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral19, falsyLiteral19, true) === true, 'choose true 19');
check(chooseBoolean(truthyLiteral19, falsyLiteral19, false) === false, 'choose false 19');
var truthyLiteral20 = true;
var falsyLiteral20 = false;
check(truthyLiteral20 === true, 'true strict 20');
check(falsyLiteral20 === false, 'false strict 20');
check(truthyLiteral20 !== falsyLiteral20, 'boolean distinct 20');
check(typeof truthyLiteral20 === 'boolean', 'true typeof 20');
check(typeof falsyLiteral20 === 'boolean', 'false typeof 20');
check((truthyLiteral20 && !falsyLiteral20) === true, 'boolean and not 20');
check((falsyLiteral20 || truthyLiteral20) === true, 'boolean or 20');
check((truthyLiteral20 ? 21 : 22) === 21, 'true conditional 20');
check((falsyLiteral20 ? 21 : 22) === 22, 'false conditional 20');
if (truthyLiteral20) { score = score + 1; } else { throw 'true branch 20'; }
if (falsyLiteral20) { throw 'false branch 20'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral20, falsyLiteral20, true) === true, 'choose true 20');
check(chooseBoolean(truthyLiteral20, falsyLiteral20, false) === false, 'choose false 20');
var truthyLiteral21 = true;
var falsyLiteral21 = false;
check(truthyLiteral21 === true, 'true strict 21');
check(falsyLiteral21 === false, 'false strict 21');
check(truthyLiteral21 !== falsyLiteral21, 'boolean distinct 21');
check(typeof truthyLiteral21 === 'boolean', 'true typeof 21');
check(typeof falsyLiteral21 === 'boolean', 'false typeof 21');
check((truthyLiteral21 && !falsyLiteral21) === true, 'boolean and not 21');
check((falsyLiteral21 || truthyLiteral21) === true, 'boolean or 21');
check((truthyLiteral21 ? 22 : 23) === 22, 'true conditional 21');
check((falsyLiteral21 ? 22 : 23) === 23, 'false conditional 21');
if (truthyLiteral21) { score = score + 1; } else { throw 'true branch 21'; }
if (falsyLiteral21) { throw 'false branch 21'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral21, falsyLiteral21, true) === true, 'choose true 21');
check(chooseBoolean(truthyLiteral21, falsyLiteral21, false) === false, 'choose false 21');
var truthyLiteral22 = true;
var falsyLiteral22 = false;
check(truthyLiteral22 === true, 'true strict 22');
check(falsyLiteral22 === false, 'false strict 22');
check(truthyLiteral22 !== falsyLiteral22, 'boolean distinct 22');
check(typeof truthyLiteral22 === 'boolean', 'true typeof 22');
check(typeof falsyLiteral22 === 'boolean', 'false typeof 22');
check((truthyLiteral22 && !falsyLiteral22) === true, 'boolean and not 22');
check((falsyLiteral22 || truthyLiteral22) === true, 'boolean or 22');
check((truthyLiteral22 ? 23 : 24) === 23, 'true conditional 22');
check((falsyLiteral22 ? 23 : 24) === 24, 'false conditional 22');
if (truthyLiteral22) { score = score + 1; } else { throw 'true branch 22'; }
if (falsyLiteral22) { throw 'false branch 22'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral22, falsyLiteral22, true) === true, 'choose true 22');
check(chooseBoolean(truthyLiteral22, falsyLiteral22, false) === false, 'choose false 22');
var truthyLiteral23 = true;
var falsyLiteral23 = false;
check(truthyLiteral23 === true, 'true strict 23');
check(falsyLiteral23 === false, 'false strict 23');
check(truthyLiteral23 !== falsyLiteral23, 'boolean distinct 23');
check(typeof truthyLiteral23 === 'boolean', 'true typeof 23');
check(typeof falsyLiteral23 === 'boolean', 'false typeof 23');
check((truthyLiteral23 && !falsyLiteral23) === true, 'boolean and not 23');
check((falsyLiteral23 || truthyLiteral23) === true, 'boolean or 23');
check((truthyLiteral23 ? 24 : 25) === 24, 'true conditional 23');
check((falsyLiteral23 ? 24 : 25) === 25, 'false conditional 23');
if (truthyLiteral23) { score = score + 1; } else { throw 'true branch 23'; }
if (falsyLiteral23) { throw 'false branch 23'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral23, falsyLiteral23, true) === true, 'choose true 23');
check(chooseBoolean(truthyLiteral23, falsyLiteral23, false) === false, 'choose false 23');
var truthyLiteral24 = true;
var falsyLiteral24 = false;
check(truthyLiteral24 === true, 'true strict 24');
check(falsyLiteral24 === false, 'false strict 24');
check(truthyLiteral24 !== falsyLiteral24, 'boolean distinct 24');
check(typeof truthyLiteral24 === 'boolean', 'true typeof 24');
check(typeof falsyLiteral24 === 'boolean', 'false typeof 24');
check((truthyLiteral24 && !falsyLiteral24) === true, 'boolean and not 24');
check((falsyLiteral24 || truthyLiteral24) === true, 'boolean or 24');
check((truthyLiteral24 ? 25 : 26) === 25, 'true conditional 24');
check((falsyLiteral24 ? 25 : 26) === 26, 'false conditional 24');
if (truthyLiteral24) { score = score + 1; } else { throw 'true branch 24'; }
if (falsyLiteral24) { throw 'false branch 24'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral24, falsyLiteral24, true) === true, 'choose true 24');
check(chooseBoolean(truthyLiteral24, falsyLiteral24, false) === false, 'choose false 24');
var truthyLiteral25 = true;
var falsyLiteral25 = false;
check(truthyLiteral25 === true, 'true strict 25');
check(falsyLiteral25 === false, 'false strict 25');
check(truthyLiteral25 !== falsyLiteral25, 'boolean distinct 25');
check(typeof truthyLiteral25 === 'boolean', 'true typeof 25');
check(typeof falsyLiteral25 === 'boolean', 'false typeof 25');
check((truthyLiteral25 && !falsyLiteral25) === true, 'boolean and not 25');
check((falsyLiteral25 || truthyLiteral25) === true, 'boolean or 25');
check((truthyLiteral25 ? 26 : 27) === 26, 'true conditional 25');
check((falsyLiteral25 ? 26 : 27) === 27, 'false conditional 25');
if (truthyLiteral25) { score = score + 1; } else { throw 'true branch 25'; }
if (falsyLiteral25) { throw 'false branch 25'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral25, falsyLiteral25, true) === true, 'choose true 25');
check(chooseBoolean(truthyLiteral25, falsyLiteral25, false) === false, 'choose false 25');
var truthyLiteral26 = true;
var falsyLiteral26 = false;
check(truthyLiteral26 === true, 'true strict 26');
check(falsyLiteral26 === false, 'false strict 26');
check(truthyLiteral26 !== falsyLiteral26, 'boolean distinct 26');
check(typeof truthyLiteral26 === 'boolean', 'true typeof 26');
check(typeof falsyLiteral26 === 'boolean', 'false typeof 26');
check((truthyLiteral26 && !falsyLiteral26) === true, 'boolean and not 26');
check((falsyLiteral26 || truthyLiteral26) === true, 'boolean or 26');
check((truthyLiteral26 ? 27 : 28) === 27, 'true conditional 26');
check((falsyLiteral26 ? 27 : 28) === 28, 'false conditional 26');
if (truthyLiteral26) { score = score + 1; } else { throw 'true branch 26'; }
if (falsyLiteral26) { throw 'false branch 26'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral26, falsyLiteral26, true) === true, 'choose true 26');
check(chooseBoolean(truthyLiteral26, falsyLiteral26, false) === false, 'choose false 26');
var truthyLiteral27 = true;
var falsyLiteral27 = false;
check(truthyLiteral27 === true, 'true strict 27');
check(falsyLiteral27 === false, 'false strict 27');
check(truthyLiteral27 !== falsyLiteral27, 'boolean distinct 27');
check(typeof truthyLiteral27 === 'boolean', 'true typeof 27');
check(typeof falsyLiteral27 === 'boolean', 'false typeof 27');
check((truthyLiteral27 && !falsyLiteral27) === true, 'boolean and not 27');
check((falsyLiteral27 || truthyLiteral27) === true, 'boolean or 27');
check((truthyLiteral27 ? 28 : 29) === 28, 'true conditional 27');
check((falsyLiteral27 ? 28 : 29) === 29, 'false conditional 27');
if (truthyLiteral27) { score = score + 1; } else { throw 'true branch 27'; }
if (falsyLiteral27) { throw 'false branch 27'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral27, falsyLiteral27, true) === true, 'choose true 27');
check(chooseBoolean(truthyLiteral27, falsyLiteral27, false) === false, 'choose false 27');
var truthyLiteral28 = true;
var falsyLiteral28 = false;
check(truthyLiteral28 === true, 'true strict 28');
check(falsyLiteral28 === false, 'false strict 28');
check(truthyLiteral28 !== falsyLiteral28, 'boolean distinct 28');
check(typeof truthyLiteral28 === 'boolean', 'true typeof 28');
check(typeof falsyLiteral28 === 'boolean', 'false typeof 28');
check((truthyLiteral28 && !falsyLiteral28) === true, 'boolean and not 28');
check((falsyLiteral28 || truthyLiteral28) === true, 'boolean or 28');
check((truthyLiteral28 ? 29 : 30) === 29, 'true conditional 28');
check((falsyLiteral28 ? 29 : 30) === 30, 'false conditional 28');
if (truthyLiteral28) { score = score + 1; } else { throw 'true branch 28'; }
if (falsyLiteral28) { throw 'false branch 28'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral28, falsyLiteral28, true) === true, 'choose true 28');
check(chooseBoolean(truthyLiteral28, falsyLiteral28, false) === false, 'choose false 28');
var truthyLiteral29 = true;
var falsyLiteral29 = false;
check(truthyLiteral29 === true, 'true strict 29');
check(falsyLiteral29 === false, 'false strict 29');
check(truthyLiteral29 !== falsyLiteral29, 'boolean distinct 29');
check(typeof truthyLiteral29 === 'boolean', 'true typeof 29');
check(typeof falsyLiteral29 === 'boolean', 'false typeof 29');
check((truthyLiteral29 && !falsyLiteral29) === true, 'boolean and not 29');
check((falsyLiteral29 || truthyLiteral29) === true, 'boolean or 29');
check((truthyLiteral29 ? 30 : 31) === 30, 'true conditional 29');
check((falsyLiteral29 ? 30 : 31) === 31, 'false conditional 29');
if (truthyLiteral29) { score = score + 1; } else { throw 'true branch 29'; }
if (falsyLiteral29) { throw 'false branch 29'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral29, falsyLiteral29, true) === true, 'choose true 29');
check(chooseBoolean(truthyLiteral29, falsyLiteral29, false) === false, 'choose false 29');
var truthyLiteral30 = true;
var falsyLiteral30 = false;
check(truthyLiteral30 === true, 'true strict 30');
check(falsyLiteral30 === false, 'false strict 30');
check(truthyLiteral30 !== falsyLiteral30, 'boolean distinct 30');
check(typeof truthyLiteral30 === 'boolean', 'true typeof 30');
check(typeof falsyLiteral30 === 'boolean', 'false typeof 30');
check((truthyLiteral30 && !falsyLiteral30) === true, 'boolean and not 30');
check((falsyLiteral30 || truthyLiteral30) === true, 'boolean or 30');
check((truthyLiteral30 ? 31 : 32) === 31, 'true conditional 30');
check((falsyLiteral30 ? 31 : 32) === 32, 'false conditional 30');
if (truthyLiteral30) { score = score + 1; } else { throw 'true branch 30'; }
if (falsyLiteral30) { throw 'false branch 30'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral30, falsyLiteral30, true) === true, 'choose true 30');
check(chooseBoolean(truthyLiteral30, falsyLiteral30, false) === false, 'choose false 30');
var truthyLiteral31 = true;
var falsyLiteral31 = false;
check(truthyLiteral31 === true, 'true strict 31');
check(falsyLiteral31 === false, 'false strict 31');
check(truthyLiteral31 !== falsyLiteral31, 'boolean distinct 31');
check(typeof truthyLiteral31 === 'boolean', 'true typeof 31');
check(typeof falsyLiteral31 === 'boolean', 'false typeof 31');
check((truthyLiteral31 && !falsyLiteral31) === true, 'boolean and not 31');
check((falsyLiteral31 || truthyLiteral31) === true, 'boolean or 31');
check((truthyLiteral31 ? 32 : 33) === 32, 'true conditional 31');
check((falsyLiteral31 ? 32 : 33) === 33, 'false conditional 31');
if (truthyLiteral31) { score = score + 1; } else { throw 'true branch 31'; }
if (falsyLiteral31) { throw 'false branch 31'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral31, falsyLiteral31, true) === true, 'choose true 31');
check(chooseBoolean(truthyLiteral31, falsyLiteral31, false) === false, 'choose false 31');
var truthyLiteral32 = true;
var falsyLiteral32 = false;
check(truthyLiteral32 === true, 'true strict 32');
check(falsyLiteral32 === false, 'false strict 32');
check(truthyLiteral32 !== falsyLiteral32, 'boolean distinct 32');
check(typeof truthyLiteral32 === 'boolean', 'true typeof 32');
check(typeof falsyLiteral32 === 'boolean', 'false typeof 32');
check((truthyLiteral32 && !falsyLiteral32) === true, 'boolean and not 32');
check((falsyLiteral32 || truthyLiteral32) === true, 'boolean or 32');
check((truthyLiteral32 ? 33 : 34) === 33, 'true conditional 32');
check((falsyLiteral32 ? 33 : 34) === 34, 'false conditional 32');
if (truthyLiteral32) { score = score + 1; } else { throw 'true branch 32'; }
if (falsyLiteral32) { throw 'false branch 32'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral32, falsyLiteral32, true) === true, 'choose true 32');
check(chooseBoolean(truthyLiteral32, falsyLiteral32, false) === false, 'choose false 32');
var truthyLiteral33 = true;
var falsyLiteral33 = false;
check(truthyLiteral33 === true, 'true strict 33');
check(falsyLiteral33 === false, 'false strict 33');
check(truthyLiteral33 !== falsyLiteral33, 'boolean distinct 33');
check(typeof truthyLiteral33 === 'boolean', 'true typeof 33');
check(typeof falsyLiteral33 === 'boolean', 'false typeof 33');
check((truthyLiteral33 && !falsyLiteral33) === true, 'boolean and not 33');
check((falsyLiteral33 || truthyLiteral33) === true, 'boolean or 33');
check((truthyLiteral33 ? 34 : 35) === 34, 'true conditional 33');
check((falsyLiteral33 ? 34 : 35) === 35, 'false conditional 33');
if (truthyLiteral33) { score = score + 1; } else { throw 'true branch 33'; }
if (falsyLiteral33) { throw 'false branch 33'; } else { score = score + 1; }
check(chooseBoolean(truthyLiteral33, falsyLiteral33, true) === true, 'choose true 33');
check(chooseBoolean(truthyLiteral33, falsyLiteral33, false) === false, 'choose false 33');
console.log('boolean-literals-evaluate-booleans large ' + score);
