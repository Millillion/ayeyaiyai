var values = [2, 1, 3];
const [[...x] = values] = [];

console.log(
  Array.isArray(x),
  x[0],
  x[1],
  x[2],
  x.length,
  x === values,
);

if (!Array.isArray(x)) {
  throw new Error("isArray");
}
if (x[0] !== 2) {
  throw new Error("x0");
}
if (x[1] !== 1) {
  throw new Error("x1");
}
if (x[2] !== 3) {
  throw new Error("x2");
}
if (x.length !== 3) {
  throw new Error("length");
}
if (x === values) {
  throw new Error("identity");
}
