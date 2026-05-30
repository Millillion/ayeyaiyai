async function * agen1() {
  yield import('./.tmp-dyn-import-yield-a.js');
  yield import('./.tmp-dyn-import-yield-poisoned.js');
}

var iter = agen1();

iter.next().then(({ value, done }) => {
  if (done !== false) {
    throw 'done-a';
  }
  if (value.x !== 42) {
    throw 'value-a';
  }
  return iter.next();
}).then(function () {
  throw 'expected-rejection';
}, function (error) {
  if (error !== 'foo') {
    throw 'error';
  }
});
