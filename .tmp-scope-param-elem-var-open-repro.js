var x = 'outside';
var probe1, probe2;

(function*(
    _ = probe1 = function() { return x; },
    __ = (eval('var x = "inside";'), probe2 = function() { return x; })
  ) {
}().next());

if (probe1 === undefined) {
  throw new Error('probe1 undefined');
}
if (probe2 === undefined) {
  throw new Error('probe2 undefined');
}
if (probe1() !== 'inside') {
  throw new Error('probe1 not inside');
}
if (probe2() !== 'inside') {
  throw new Error('probe2 not inside');
}
