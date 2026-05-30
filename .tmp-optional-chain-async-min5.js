function __assertSameValue(actual, expected) {
  if (actual !== expected) {
    throw "fail";
  }
}

const c = {d: Promise.resolve(11)};

async function checkAssertions() {
  c.e = Promise.resolve(39);
  __assertSameValue(await c?.e, 39);
}

checkAssertions().then(function () {}, function (error) { throw error; });
