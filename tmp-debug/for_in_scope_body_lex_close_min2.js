let x = 'outside';
var probeBody;

for (let [x] in { i: 0 })
  probeBody = function() { return x; };

console.log(probeBody());
console.log(x);
