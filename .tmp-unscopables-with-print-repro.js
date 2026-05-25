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
      console.log('first', v, count);
    }
    count++;
    var v = x;
    with (globalThis) {
      count++;
      console.log('second-before', v, count);
      v = 20;
      console.log('second-after', v, globalThis.v);
    }
    console.log('third', v, globalThis.v, count);
    callCount = callCount + 1;
  };

  ref(10).next();

  console.log('after', callCount, count);

  count++;
}
console.log('final', count, v, globalThis.v);
