this.y = 1;
if ((delete this.y) !== true) {
  console.log("bad1");
}
if (this.y !== undefined) {
  console.log("bad2");
}
console.log("done");
