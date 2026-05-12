let x = 'outside';
var probeDecl, probeBody;

for (
    let [x, _ = probeDecl = function() { return x; }]
    in
    { i: 0 }
  )
  probeBody = function() { return x; };

console.log(probeDecl());
console.log(probeBody());
console.log(x);
