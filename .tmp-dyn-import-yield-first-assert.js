function same(actual, expected, name) {
  if (actual !== expected) {
    throw name;
  }
}

async function * agen1() {
  yield import('./.tmp-dyn-import-yield-a.js');
}

var aiter1 = agen1();

async function fn() {
  var a = aiter1.next();
  same((await a).value.x, 42, 'a');
}

fn().then(function () {}, function (error) { throw error; });
