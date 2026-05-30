function same(actual, expected, name) {
  if (actual !== expected) {
    throw name;
  }
}

async function * agen1() {
  yield import('./.tmp-dyn-import-yield-a.js');
  yield import('./.tmp-dyn-import-yield-a.js');
  yield import('./.tmp-dyn-import-yield-poisoned.js');
}

async function * agen2() {
  yield await import('./.tmp-dyn-import-yield-a.js');
  yield await import('./.tmp-dyn-import-yield-a.js');
  yield await import('./.tmp-dyn-import-yield-poisoned.js');
}

var aiter1 = agen1();
var aiter2 = agen2();

async function fn() {
  var a = aiter1.next();
  var b = aiter1.next();
  var c = aiter1.next();
  var d = aiter2.next();
  var e = aiter2.next();
  var f = aiter2.next();

  same((await a).value.x, 42, 'a');
  same((await b).value.x, 42, 'b');

  var error;
  try {
    await c;
  } catch (err) {
    error = err;
  }
  same(error, 'foo', 'c');

  same((await d).value.x, 42, 'd');
  same((await e).value.x, 42, 'e');

  error = null;
  try {
    await f;
  } catch (err) {
    error = err;
  }
  same(error, 'foo', 'f');
}

fn().then(function () {}, function (error) { throw error; });
