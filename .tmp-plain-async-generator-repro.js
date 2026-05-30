var seen = 0;

async function *g() {
  return 42;
}

g().next().then(function(result) {
  if (result.value !== 42) {
    throw new Error("bad value");
  }
  if (result.done !== true) {
    throw new Error("bad done");
  }
  seen = seen + 1;
});

if (seen !== 1) {
  throw new Error("callback did not run");
}
