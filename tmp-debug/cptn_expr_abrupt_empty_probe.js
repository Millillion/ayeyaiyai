console.log(eval('var a; 1; for (a in { x: 0 }) { break; }'));
console.log(eval('var b; 2; for (b in { x: 0 }) { 3; break; }'));
console.log(eval('var c; 4; outer: do { for (c in { x: 0 }) { continue outer; } } while (false)'));
console.log(eval('var d; 5; outer: do { for (d in { x: 0 }) { 6; continue outer; } } while (false)'));
