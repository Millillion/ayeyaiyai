function verifyProperty(obj, name, desc) {
  if (obj[name] !== desc.value) {
    throw new Error();
  }
}

var arrow;
var counter = 0;

for ([arrow = () => {}] of [[]]) {
  verifyProperty(arrow, "name", {
    enumerable: false,
    writable: false,
    configurable: true,
    value: "arrow",
  });
  counter += 1;
}

if (counter !== 1) {
  throw new Error();
}
