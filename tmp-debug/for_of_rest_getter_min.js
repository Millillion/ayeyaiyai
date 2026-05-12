var count = 0;
var iterCount = 0;

for (const {...x} of [{ get v() { count++; return 2; } }]) {
  console.log("body", count, x.v);
  iterCount += 1;
}

console.log("done", count, iterCount);
