let count = 0;
var v = 1;
globalThis[Symbol.unscopables] = {
  v: true,
};

{
  count++;

  var callCount = 0;
  var ref;
  ref = function*(x) {
    count++;
    with (globalThis) {
      count++;
      if (v !== undefined) throw new Error('first v');
    }
    count++;
    var v = x;
    with (globalThis) {
      count++;
      if (v !== 10) throw new Error('second v');
      v = 20;
    }
    if (v !== 20) throw new Error('third v');
    if (globalThis.v !== 1) throw new Error('global v');
    callCount = callCount + 1;
  };

  ref(10).next();

  if (callCount !== 1) throw new Error('callCount');

  count++;
}
if (count !== 6) throw new Error('count');
