console.log(eval('1; for (var a in { x: 0 }) { break; }'));
console.log(eval('2; for (var b in { x: 0 }) { 3; break; }'));
console.log(eval('4; outer: do { for (var c in { x: 0 }) { continue outer; } } while (false)'));
console.log(eval('5; outer: do { for (var d in { x: 0 }) { 6; continue outer; } } while (false)'));
