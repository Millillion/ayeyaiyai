function __assertSameValue(actual, expected) {
  if (actual !== expected) {
    throw "fail";
  }
}

const a = undefined;
const c = {d: Promise.resolve(11)};

async function checkAssertions() {
  __assertSameValue(await a?.b, undefined);
  __assertSameValue(await c?.d, 11);

  Promise.prototype.x = 42;
  var res = await Promise.resolve(undefined)?.x;
  __assertSameValue(res, 42);

  Promise.prototype.y = 43;
  var res = await Promise.reject(undefined)?.y;
  __assertSameValue(res, 43);

  c.e = Promise.resolve(39);
  __assertSameValue(await c?.e, 39);
}

checkAssertions().then(function () {}, function (error) { throw error; });
