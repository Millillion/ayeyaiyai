// behavior: spread-elements-append-iterator-values
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
function makeIterable(base, count) {
return {
[Symbol.iterator]: function() {
var index = 0;
return {
next: function() {
if (index < count) {
var value = base + index;
index = index + 1;
return { value: value, done: false };
}
return { value: base + count, done: true };
}
};
}
};
}
var iterableSpread0 = [3, ...makeIterable(4, 3), 8];
check(iterableSpread0.length === 5, 'iterable spread length 0');
check(iterableSpread0[0] === 3, 'iterable spread leading 0');
check(iterableSpread0[1] === 4 && iterableSpread0[2] === 5 && iterableSpread0[3] === 6, 'iterable spread values 0');
check(iterableSpread0[4] === 8, 'iterable spread trailing 0');
var sparseSource0 = [, 10];
var sparseSpread0 = [...sparseSource0];
check(sparseSpread0.length === 2, 'sparse spread length 0');
check((0 in sparseSpread0) && sparseSpread0[0] === undefined, 'sparse spread undefined 0');
check((1 in sparseSpread0) && sparseSpread0[1] === 10, 'sparse spread value 0');
var iterableSpread1 = [8, ...makeIterable(9, 3), 13];
check(iterableSpread1.length === 5, 'iterable spread length 1');
check(iterableSpread1[0] === 8, 'iterable spread leading 1');
check(iterableSpread1[1] === 9 && iterableSpread1[2] === 10 && iterableSpread1[3] === 11, 'iterable spread values 1');
check(iterableSpread1[4] === 13, 'iterable spread trailing 1');
var sparseSource1 = [, 15];
var sparseSpread1 = [...sparseSource1];
check(sparseSpread1.length === 2, 'sparse spread length 1');
check((0 in sparseSpread1) && sparseSpread1[0] === undefined, 'sparse spread undefined 1');
check((1 in sparseSpread1) && sparseSpread1[1] === 15, 'sparse spread value 1');
var iterableSpread2 = [13, ...makeIterable(14, 3), 18];
check(iterableSpread2.length === 5, 'iterable spread length 2');
check(iterableSpread2[0] === 13, 'iterable spread leading 2');
check(iterableSpread2[1] === 14 && iterableSpread2[2] === 15 && iterableSpread2[3] === 16, 'iterable spread values 2');
check(iterableSpread2[4] === 18, 'iterable spread trailing 2');
var sparseSource2 = [, 20];
var sparseSpread2 = [...sparseSource2];
check(sparseSpread2.length === 2, 'sparse spread length 2');
check((0 in sparseSpread2) && sparseSpread2[0] === undefined, 'sparse spread undefined 2');
check((1 in sparseSpread2) && sparseSpread2[1] === 20, 'sparse spread value 2');
var iterableSpread3 = [18, ...makeIterable(19, 3), 23];
check(iterableSpread3.length === 5, 'iterable spread length 3');
check(iterableSpread3[0] === 18, 'iterable spread leading 3');
check(iterableSpread3[1] === 19 && iterableSpread3[2] === 20 && iterableSpread3[3] === 21, 'iterable spread values 3');
check(iterableSpread3[4] === 23, 'iterable spread trailing 3');
var sparseSource3 = [, 25];
var sparseSpread3 = [...sparseSource3];
check(sparseSpread3.length === 2, 'sparse spread length 3');
check((0 in sparseSpread3) && sparseSpread3[0] === undefined, 'sparse spread undefined 3');
check((1 in sparseSpread3) && sparseSpread3[1] === 25, 'sparse spread value 3');
var iterableSpread4 = [23, ...makeIterable(24, 3), 28];
check(iterableSpread4.length === 5, 'iterable spread length 4');
check(iterableSpread4[0] === 23, 'iterable spread leading 4');
check(iterableSpread4[1] === 24 && iterableSpread4[2] === 25 && iterableSpread4[3] === 26, 'iterable spread values 4');
check(iterableSpread4[4] === 28, 'iterable spread trailing 4');
var sparseSource4 = [, 30];
var sparseSpread4 = [...sparseSource4];
check(sparseSpread4.length === 2, 'sparse spread length 4');
check((0 in sparseSpread4) && sparseSpread4[0] === undefined, 'sparse spread undefined 4');
check((1 in sparseSpread4) && sparseSpread4[1] === 30, 'sparse spread value 4');
var iterableSpread5 = [28, ...makeIterable(29, 3), 33];
check(iterableSpread5.length === 5, 'iterable spread length 5');
check(iterableSpread5[0] === 28, 'iterable spread leading 5');
check(iterableSpread5[1] === 29 && iterableSpread5[2] === 30 && iterableSpread5[3] === 31, 'iterable spread values 5');
check(iterableSpread5[4] === 33, 'iterable spread trailing 5');
var sparseSource5 = [, 35];
var sparseSpread5 = [...sparseSource5];
check(sparseSpread5.length === 2, 'sparse spread length 5');
check((0 in sparseSpread5) && sparseSpread5[0] === undefined, 'sparse spread undefined 5');
check((1 in sparseSpread5) && sparseSpread5[1] === 35, 'sparse spread value 5');
var iterableSpread6 = [33, ...makeIterable(34, 3), 38];
check(iterableSpread6.length === 5, 'iterable spread length 6');
check(iterableSpread6[0] === 33, 'iterable spread leading 6');
check(iterableSpread6[1] === 34 && iterableSpread6[2] === 35 && iterableSpread6[3] === 36, 'iterable spread values 6');
check(iterableSpread6[4] === 38, 'iterable spread trailing 6');
var sparseSource6 = [, 40];
var sparseSpread6 = [...sparseSource6];
check(sparseSpread6.length === 2, 'sparse spread length 6');
check((0 in sparseSpread6) && sparseSpread6[0] === undefined, 'sparse spread undefined 6');
check((1 in sparseSpread6) && sparseSpread6[1] === 40, 'sparse spread value 6');
var iterableSpread7 = [38, ...makeIterable(39, 3), 43];
check(iterableSpread7.length === 5, 'iterable spread length 7');
check(iterableSpread7[0] === 38, 'iterable spread leading 7');
check(iterableSpread7[1] === 39 && iterableSpread7[2] === 40 && iterableSpread7[3] === 41, 'iterable spread values 7');
check(iterableSpread7[4] === 43, 'iterable spread trailing 7');
var sparseSource7 = [, 45];
var sparseSpread7 = [...sparseSource7];
check(sparseSpread7.length === 2, 'sparse spread length 7');
check((0 in sparseSpread7) && sparseSpread7[0] === undefined, 'sparse spread undefined 7');
check((1 in sparseSpread7) && sparseSpread7[1] === 45, 'sparse spread value 7');
var iterableSpread8 = [43, ...makeIterable(44, 3), 48];
check(iterableSpread8.length === 5, 'iterable spread length 8');
check(iterableSpread8[0] === 43, 'iterable spread leading 8');
check(iterableSpread8[1] === 44 && iterableSpread8[2] === 45 && iterableSpread8[3] === 46, 'iterable spread values 8');
check(iterableSpread8[4] === 48, 'iterable spread trailing 8');
var sparseSource8 = [, 50];
var sparseSpread8 = [...sparseSource8];
check(sparseSpread8.length === 2, 'sparse spread length 8');
check((0 in sparseSpread8) && sparseSpread8[0] === undefined, 'sparse spread undefined 8');
check((1 in sparseSpread8) && sparseSpread8[1] === 50, 'sparse spread value 8');
console.log('spread-elements-append-iterator-values standard ' + score);
