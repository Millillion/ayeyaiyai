function __assertSameValue(actual, expected) {
  if (actual !== expected) {
    throw "fail";
  }
}

const c = {d: Promise.resolve(11)};

async function checkAssertions() {
  __assertSameValue(await c?.d, 11);
}

checkAssertions().then(function () {}, function (error) { throw error; });
