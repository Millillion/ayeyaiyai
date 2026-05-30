var rab = new ArrayBuffer(4, { maxByteLength: 8 });
var fixedLength = new Uint8Array(rab, 0, 4);
var ta_write = new Uint8Array(rab);

for (var i = 0; i < 4; ++i) {
  ta_write[i] = i;
}

var a, b, c, d, e;
[a, b, c, d, e] = fixedLength;
if (a !== 0 || b !== 1 || c !== 2 || d !== 3 || e !== undefined) {
  throw new Error("fixedLength");
}
