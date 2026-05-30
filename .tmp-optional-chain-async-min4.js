function __assertSameValue(actual, expected) {
  if (actual !== expected) {
    throw "fail";
  }
}

async function checkAssertions() {
  Promise.prototype.y = 43;
  var res = await Promise.reject(undefined)?.y;
  __assertSameValue(res, 43);
}

checkAssertions().then(function () {}, function (error) { throw error; });
