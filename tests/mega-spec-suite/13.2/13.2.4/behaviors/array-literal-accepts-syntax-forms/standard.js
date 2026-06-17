// behavior: array-literal-accepts-syntax-forms
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
function same(value) { return value; }
function* syntaxGenerator(seed) {
var yielded = yield seed;
return [yielded, ...[seed + 1], , seed + 3,];
}
async function syntaxAsync(seed) {
return [await Promise.resolve(seed), ...[seed + 1], , seed + 3,];
}
check(typeof syntaxAsync === 'function', 'await syntax function');
var syntaxIter = syntaxGenerator(10);
var syntaxFirst = syntaxIter.next();
check(syntaxFirst.value === 10 && syntaxFirst.done === false, 'yield syntax first');
var syntaxDone = syntaxIter.next(20);
check(syntaxDone.done === true, 'yield syntax done');
check(syntaxDone.value.length === 4, 'yield syntax length');
check(syntaxDone.value[0] === 20 && syntaxDone.value[1] === 11 && !(2 in syntaxDone.value) && syntaxDone.value[3] === 13, 'yield syntax values');
var emptySyntax0 = [];
check(emptySyntax0.length === 0, 'empty syntax 0');
var elisionSyntax0 = [,,];
check(elisionSyntax0.length === 2, 'elision syntax length 0');
check(!(0 in elisionSyntax0) && !(1 in elisionSyntax0), 'elision syntax holes 0');
var elementSyntax0 = [1, same(2), ...[3, 4], , 6,];
check(elementSyntax0.length === 6, 'element syntax length 0');
check(elementSyntax0[0] === 1 && elementSyntax0[1] === 2, 'element syntax prefix 0');
check(elementSyntax0[2] === 3 && elementSyntax0[3] === 4, 'element syntax spread 0');
check(!(4 in elementSyntax0) && elementSyntax0[5] === 6, 'element syntax hole tail 0');
var emptySyntax1 = [];
check(emptySyntax1.length === 0, 'empty syntax 1');
var elisionSyntax1 = [,,];
check(elisionSyntax1.length === 2, 'elision syntax length 1');
check(!(0 in elisionSyntax1) && !(1 in elisionSyntax1), 'elision syntax holes 1');
var elementSyntax1 = [2, same(3), ...[4, 5], , 7,];
check(elementSyntax1.length === 6, 'element syntax length 1');
check(elementSyntax1[0] === 2 && elementSyntax1[1] === 3, 'element syntax prefix 1');
check(elementSyntax1[2] === 4 && elementSyntax1[3] === 5, 'element syntax spread 1');
check(!(4 in elementSyntax1) && elementSyntax1[5] === 7, 'element syntax hole tail 1');
var emptySyntax2 = [];
check(emptySyntax2.length === 0, 'empty syntax 2');
var elisionSyntax2 = [,,];
check(elisionSyntax2.length === 2, 'elision syntax length 2');
check(!(0 in elisionSyntax2) && !(1 in elisionSyntax2), 'elision syntax holes 2');
var elementSyntax2 = [3, same(4), ...[5, 6], , 8,];
check(elementSyntax2.length === 6, 'element syntax length 2');
check(elementSyntax2[0] === 3 && elementSyntax2[1] === 4, 'element syntax prefix 2');
check(elementSyntax2[2] === 5 && elementSyntax2[3] === 6, 'element syntax spread 2');
check(!(4 in elementSyntax2) && elementSyntax2[5] === 8, 'element syntax hole tail 2');
var emptySyntax3 = [];
check(emptySyntax3.length === 0, 'empty syntax 3');
var elisionSyntax3 = [,,];
check(elisionSyntax3.length === 2, 'elision syntax length 3');
check(!(0 in elisionSyntax3) && !(1 in elisionSyntax3), 'elision syntax holes 3');
var elementSyntax3 = [4, same(5), ...[6, 7], , 9,];
check(elementSyntax3.length === 6, 'element syntax length 3');
check(elementSyntax3[0] === 4 && elementSyntax3[1] === 5, 'element syntax prefix 3');
check(elementSyntax3[2] === 6 && elementSyntax3[3] === 7, 'element syntax spread 3');
check(!(4 in elementSyntax3) && elementSyntax3[5] === 9, 'element syntax hole tail 3');
var emptySyntax4 = [];
check(emptySyntax4.length === 0, 'empty syntax 4');
var elisionSyntax4 = [,,];
check(elisionSyntax4.length === 2, 'elision syntax length 4');
check(!(0 in elisionSyntax4) && !(1 in elisionSyntax4), 'elision syntax holes 4');
var elementSyntax4 = [5, same(6), ...[7, 8], , 10,];
check(elementSyntax4.length === 6, 'element syntax length 4');
check(elementSyntax4[0] === 5 && elementSyntax4[1] === 6, 'element syntax prefix 4');
check(elementSyntax4[2] === 7 && elementSyntax4[3] === 8, 'element syntax spread 4');
check(!(4 in elementSyntax4) && elementSyntax4[5] === 10, 'element syntax hole tail 4');
var emptySyntax5 = [];
check(emptySyntax5.length === 0, 'empty syntax 5');
var elisionSyntax5 = [,,];
check(elisionSyntax5.length === 2, 'elision syntax length 5');
check(!(0 in elisionSyntax5) && !(1 in elisionSyntax5), 'elision syntax holes 5');
var elementSyntax5 = [6, same(7), ...[8, 9], , 11,];
check(elementSyntax5.length === 6, 'element syntax length 5');
check(elementSyntax5[0] === 6 && elementSyntax5[1] === 7, 'element syntax prefix 5');
check(elementSyntax5[2] === 8 && elementSyntax5[3] === 9, 'element syntax spread 5');
check(!(4 in elementSyntax5) && elementSyntax5[5] === 11, 'element syntax hole tail 5');
var emptySyntax6 = [];
check(emptySyntax6.length === 0, 'empty syntax 6');
var elisionSyntax6 = [,,];
check(elisionSyntax6.length === 2, 'elision syntax length 6');
check(!(0 in elisionSyntax6) && !(1 in elisionSyntax6), 'elision syntax holes 6');
var elementSyntax6 = [7, same(8), ...[9, 10], , 12,];
check(elementSyntax6.length === 6, 'element syntax length 6');
check(elementSyntax6[0] === 7 && elementSyntax6[1] === 8, 'element syntax prefix 6');
check(elementSyntax6[2] === 9 && elementSyntax6[3] === 10, 'element syntax spread 6');
check(!(4 in elementSyntax6) && elementSyntax6[5] === 12, 'element syntax hole tail 6');
var emptySyntax7 = [];
check(emptySyntax7.length === 0, 'empty syntax 7');
var elisionSyntax7 = [,,];
check(elisionSyntax7.length === 2, 'elision syntax length 7');
check(!(0 in elisionSyntax7) && !(1 in elisionSyntax7), 'elision syntax holes 7');
var elementSyntax7 = [8, same(9), ...[10, 11], , 13,];
check(elementSyntax7.length === 6, 'element syntax length 7');
check(elementSyntax7[0] === 8 && elementSyntax7[1] === 9, 'element syntax prefix 7');
check(elementSyntax7[2] === 10 && elementSyntax7[3] === 11, 'element syntax spread 7');
check(!(4 in elementSyntax7) && elementSyntax7[5] === 13, 'element syntax hole tail 7');
var emptySyntax8 = [];
check(emptySyntax8.length === 0, 'empty syntax 8');
var elisionSyntax8 = [,,];
check(elisionSyntax8.length === 2, 'elision syntax length 8');
check(!(0 in elisionSyntax8) && !(1 in elisionSyntax8), 'elision syntax holes 8');
var elementSyntax8 = [9, same(10), ...[11, 12], , 14,];
check(elementSyntax8.length === 6, 'element syntax length 8');
check(elementSyntax8[0] === 9 && elementSyntax8[1] === 10, 'element syntax prefix 8');
check(elementSyntax8[2] === 11 && elementSyntax8[3] === 12, 'element syntax spread 8');
check(!(4 in elementSyntax8) && elementSyntax8[5] === 14, 'element syntax hole tail 8');
console.log('array-literal-accepts-syntax-forms standard ' + score);
