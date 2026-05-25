if (this.x !== undefined) {
  console.log("initial-bad");
}

this.x++;

if (x === undefined) {
  console.log("bad");
} else {
  console.log("ok");
}
