function $DONE(error) {
  if (error !== undefined) {
    throw error;
  }
}

function asyncTest(testFunc) {
  try {
    testFunc().then(
      function () {
        $DONE();
      },
      function (error) {
        $DONE(error);
      }
    );
  } catch (syncError) {
    $DONE(syncError);
  }
}

function __assertSameValue(actual, expected, name) {
  if (actual !== expected) {
    throw name;
  }
}

async function * agen1() {
  yield import('./.tmp-dyn-import-yield-a.js');
  yield import('./.tmp-dyn-import-yield-b.js');
  yield import('./.tmp-dyn-import-yield-poisoned.js');
}

async function * agen2() {
  yield await import('./.tmp-dyn-import-yield-a.js');
  yield await import('./.tmp-dyn-import-yield-b.js');
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

  __assertSameValue((await a).value.x, 42, 'a');
  __assertSameValue((await b).value.x, 39, 'b');

  var error;
  try {
    await c;
  } catch (err) {
    error = err;
  }
  __assertSameValue(error, 'foo', 'c');

  __assertSameValue((await d).value.x, 42, 'd');
  __assertSameValue((await e).value.x, 39, 'e');

  error = null;
  try {
    await f;
  } catch (err) {
    error = err;
  }
  __assertSameValue(error, 'foo', 'f');
}

asyncTest(fn);
