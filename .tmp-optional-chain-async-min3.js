function __assertSameValue(actual, expected) {
  if (actual !== expected) {
    throw "fail";
  }
}

async function checkAssertions() {
  Promise.prototype.x = 42;
  var res = await Promise.resolve(undefined)?.x;
  __assertSameValue(res, 42);
}

checkAssertions().then(function () {}, function (error) { throw error; });
