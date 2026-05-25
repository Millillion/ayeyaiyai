var x = 'outside';
var probe1, probe2;

(function*(
    _ = probe1 = function() { return x; },
    __ = (eval('var x = "inside";'), probe2 = function() { return x; })
  ) {
}().next());

assert.sameValue(probe1(), 'inside');
assert.sameValue(probe2(), 'inside');
