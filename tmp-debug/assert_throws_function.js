function f() { throw new ReferenceError(); }
assert.throws(ReferenceError, function() { f(); });
