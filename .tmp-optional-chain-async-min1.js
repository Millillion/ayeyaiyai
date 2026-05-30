function __assertSameValue(actual, expected) {
  if (actual !== expected) {
    throw "fail";
  }
}

const a = undefined;

async function checkAssertions() {
  __assertSameValue(await a?.b, undefined);
}

checkAssertions().then(function () {}, function (error) { throw error; });
