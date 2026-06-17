// behavior: boolean-literals-evaluate-booleans
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
console.log('boolean-literals-evaluate-booleans standard ' + score);
