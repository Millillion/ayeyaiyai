let x = 'outside';
var probeFirst, probeSecond;

for (let x in { a: 0, b: 0 })
  if (!probeFirst)
    probeFirst = function() { return x; };
  else
    probeSecond = function() { return x; };

console.log(probeFirst());
console.log(probeSecond());
console.log(probeFirst() === probeSecond());
