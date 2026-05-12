var x = 'outside';
var probe1, probe2, probeBody;

((
    _ = (eval('var x = "inside";'), probe1 = function() { return x; }),
    __ = probe2 = function() { return x; }
  ) => {
  probeBody = function() { return x; };
})();

if (probe1() !== 'inside') {
  throw 'probe1';
}
if (probe2() !== 'inside') {
  throw 'probe2';
}
if (probeBody() !== 'inside') {
  throw 'probeBody';
}
