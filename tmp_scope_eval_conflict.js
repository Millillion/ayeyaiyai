var a = () => { let x; eval('var x;'); };

assert.throws(SyntaxError, a);
