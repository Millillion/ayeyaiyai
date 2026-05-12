async function main() {
  console.log("start");
  const obj1 = { async [Symbol.asyncDispose]() { } };
  console.log("obj1");
  const obj2 = { async [Symbol.asyncDispose]() { } };
  const obj3 = { async [Symbol.asyncDispose]() { } };
  let i = 0;
  let f = [undefined, undefined, undefined];
  console.log("before loop");
  for (await using x of [obj1, obj2, obj3]) {
    console.log("loop");
    f[i++] = function() { return x; };
  }
  console.log("after loop");
  console.log(f[0]());
}
main();
