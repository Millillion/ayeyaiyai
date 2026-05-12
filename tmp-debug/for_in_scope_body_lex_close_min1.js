let x = 'outside';
var probeDecl;

for (let [x, _ = probeDecl = function() { return x; }] in { i: 0 }) {
}

console.log(probeDecl());
console.log(x);
