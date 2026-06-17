// behavior: primary-identifier-reference-unresolvable-throws
// expected: runtime-error
// goal: script
// size: standard
// variant: script.sloppy

var setupPrimaryTotal = 0;
var setupPrimaryBase = 11;
function checkSetupPrimary(condition, label) {
if (!condition) {
throw label;
}
setupPrimaryTotal = setupPrimaryTotal + 1;
return true;
}
function setupPrimaryReferenceWork() {
var setupPrimaryBase = 20;
var localTotal = 0;
localTotal = localTotal + setupPrimaryBase;
checkSetupPrimary(localTotal === 20, "setup local base");
var setupLocal0 = setupPrimaryBase + 0;
checkSetupPrimary(setupLocal0 === setupPrimaryBase + 0, "setup read 0");
setupLocal0 = setupLocal0 + 1;
checkSetupPrimary(setupLocal0 === setupPrimaryBase + 0 + 1, "setup write 0");
localTotal = localTotal + setupLocal0;
var setupBox0 = { value: setupLocal0, base: setupPrimaryBase };
checkSetupPrimary(setupBox0.base === setupPrimaryBase, "setup object 0");
var setupList0 = [setupPrimaryBase, setupLocal0];
checkSetupPrimary(setupList0[0] + setupList0[1] === setupPrimaryBase + setupLocal0, "setup array 0");
var setupLocal1 = setupPrimaryBase + 1;
checkSetupPrimary(setupLocal1 === setupPrimaryBase + 1, "setup read 1");
setupLocal1 = setupLocal1 + 2;
checkSetupPrimary(setupLocal1 === setupPrimaryBase + 1 + 2, "setup write 1");
localTotal = localTotal + setupLocal1;
var setupLocal2 = setupPrimaryBase + 2;
checkSetupPrimary(setupLocal2 === setupPrimaryBase + 2, "setup read 2");
setupLocal2 = setupLocal2 + 3;
checkSetupPrimary(setupLocal2 === setupPrimaryBase + 2 + 3, "setup write 2");
localTotal = localTotal + setupLocal2;
var setupLocal3 = setupPrimaryBase + 3;
checkSetupPrimary(setupLocal3 === setupPrimaryBase + 3, "setup read 3");
setupLocal3 = setupLocal3 + 4;
checkSetupPrimary(setupLocal3 === setupPrimaryBase + 3 + 4, "setup write 3");
localTotal = localTotal + setupLocal3;
var setupBox3 = { value: setupLocal3, base: setupPrimaryBase };
checkSetupPrimary(setupBox3.base === setupPrimaryBase, "setup object 3");
var setupLocal4 = setupPrimaryBase + 4;
checkSetupPrimary(setupLocal4 === setupPrimaryBase + 4, "setup read 4");
setupLocal4 = setupLocal4 + 5;
checkSetupPrimary(setupLocal4 === setupPrimaryBase + 4 + 5, "setup write 4");
localTotal = localTotal + setupLocal4;
var setupList4 = [setupPrimaryBase, setupLocal4];
checkSetupPrimary(setupList4[0] + setupList4[1] === setupPrimaryBase + setupLocal4, "setup array 4");
var setupLocal5 = setupPrimaryBase + 5;
checkSetupPrimary(setupLocal5 === setupPrimaryBase + 5, "setup read 5");
setupLocal5 = setupLocal5 + 6;
checkSetupPrimary(setupLocal5 === setupPrimaryBase + 5 + 6, "setup write 5");
localTotal = localTotal + setupLocal5;
var setupLocal6 = setupPrimaryBase + 6;
checkSetupPrimary(setupLocal6 === setupPrimaryBase + 6, "setup read 6");
setupLocal6 = setupLocal6 + 7;
checkSetupPrimary(setupLocal6 === setupPrimaryBase + 6 + 7, "setup write 6");
localTotal = localTotal + setupLocal6;
var setupBox6 = { value: setupLocal6, base: setupPrimaryBase };
checkSetupPrimary(setupBox6.base === setupPrimaryBase, "setup object 6");
var setupLocal7 = setupPrimaryBase + 7;
checkSetupPrimary(setupLocal7 === setupPrimaryBase + 7, "setup read 7");
setupLocal7 = setupLocal7 + 8;
checkSetupPrimary(setupLocal7 === setupPrimaryBase + 7 + 8, "setup write 7");
localTotal = localTotal + setupLocal7;
var setupLocal8 = setupPrimaryBase + 8;
checkSetupPrimary(setupLocal8 === setupPrimaryBase + 8, "setup read 8");
setupLocal8 = setupLocal8 + 9;
checkSetupPrimary(setupLocal8 === setupPrimaryBase + 8 + 9, "setup write 8");
localTotal = localTotal + setupLocal8;
var setupList8 = [setupPrimaryBase, setupLocal8];
checkSetupPrimary(setupList8[0] + setupList8[1] === setupPrimaryBase + setupLocal8, "setup array 8");
var setupLocal9 = setupPrimaryBase + 9;
checkSetupPrimary(setupLocal9 === setupPrimaryBase + 9, "setup read 9");
setupLocal9 = setupLocal9 + 1;
checkSetupPrimary(setupLocal9 === setupPrimaryBase + 9 + 1, "setup write 9");
localTotal = localTotal + setupLocal9;
var setupBox9 = { value: setupLocal9, base: setupPrimaryBase };
checkSetupPrimary(setupBox9.base === setupPrimaryBase, "setup object 9");
var setupLocal10 = setupPrimaryBase + 10;
checkSetupPrimary(setupLocal10 === setupPrimaryBase + 10, "setup read 10");
setupLocal10 = setupLocal10 + 2;
checkSetupPrimary(setupLocal10 === setupPrimaryBase + 10 + 2, "setup write 10");
localTotal = localTotal + setupLocal10;
var setupLocal11 = setupPrimaryBase + 11;
checkSetupPrimary(setupLocal11 === setupPrimaryBase + 11, "setup read 11");
setupLocal11 = setupLocal11 + 3;
checkSetupPrimary(setupLocal11 === setupPrimaryBase + 11 + 3, "setup write 11");
localTotal = localTotal + setupLocal11;
var setupLocal12 = setupPrimaryBase + 12;
checkSetupPrimary(setupLocal12 === setupPrimaryBase + 12, "setup read 12");
setupLocal12 = setupLocal12 + 4;
checkSetupPrimary(setupLocal12 === setupPrimaryBase + 12 + 4, "setup write 12");
localTotal = localTotal + setupLocal12;
var setupBox12 = { value: setupLocal12, base: setupPrimaryBase };
checkSetupPrimary(setupBox12.base === setupPrimaryBase, "setup object 12");
var setupList12 = [setupPrimaryBase, setupLocal12];
checkSetupPrimary(setupList12[0] + setupList12[1] === setupPrimaryBase + setupLocal12, "setup array 12");
var setupLocal13 = setupPrimaryBase + 13;
checkSetupPrimary(setupLocal13 === setupPrimaryBase + 13, "setup read 13");
setupLocal13 = setupLocal13 + 5;
checkSetupPrimary(setupLocal13 === setupPrimaryBase + 13 + 5, "setup write 13");
localTotal = localTotal + setupLocal13;
var setupLocal14 = setupPrimaryBase + 14;
checkSetupPrimary(setupLocal14 === setupPrimaryBase + 14, "setup read 14");
setupLocal14 = setupLocal14 + 6;
checkSetupPrimary(setupLocal14 === setupPrimaryBase + 14 + 6, "setup write 14");
localTotal = localTotal + setupLocal14;
var setupLocal15 = setupPrimaryBase + 15;
checkSetupPrimary(setupLocal15 === setupPrimaryBase + 15, "setup read 15");
setupLocal15 = setupLocal15 + 7;
checkSetupPrimary(setupLocal15 === setupPrimaryBase + 15 + 7, "setup write 15");
localTotal = localTotal + setupLocal15;
var setupBox15 = { value: setupLocal15, base: setupPrimaryBase };
checkSetupPrimary(setupBox15.base === setupPrimaryBase, "setup object 15");
return localTotal;
}
setupPrimaryTotal = setupPrimaryReferenceWork();
if (setupPrimaryTotal <= 0) { throw "setup did not run"; }
console.log(__ayyMissingPrimaryIdentifierReference);
