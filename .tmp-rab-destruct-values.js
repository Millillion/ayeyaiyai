function CreateResizableArrayBuffer(byteLength, maxByteLength) {
  return new ArrayBuffer(byteLength, { maxByteLength: maxByteLength });
}

var ctor = Uint8Array;
var rab = CreateResizableArrayBuffer(4 * ctor.BYTES_PER_ELEMENT, 8 * ctor.BYTES_PER_ELEMENT);
var fixedLength = new ctor(rab, 0, 4);
var fixedLengthWithOffset = new ctor(rab, 2 * ctor.BYTES_PER_ELEMENT, 2);
var lengthTracking = new ctor(rab, 0);
var lengthTrackingWithOffset = new ctor(rab, 2 * ctor.BYTES_PER_ELEMENT);
var ta_write = new ctor(rab);

for (var i = 0; i < 4; ++i) {
  ta_write[i] = i;
}

var a, b, c, d, e;
[a, b, c, d, e] = fixedLength;
if (a !== 0 || b !== 1 || c !== 2 || d !== 3 || e !== undefined) {
  throw new Error("fixedLength");
}

[a, b, c] = fixedLengthWithOffset;
if (a !== 2 || b !== 3 || c !== undefined) {
  throw new Error("fixedLengthWithOffset");
}

[a, b, c, d, e] = lengthTracking;
if (a !== 0 || b !== 1 || c !== 2 || d !== 3 || e !== undefined) {
  throw new Error("lengthTracking");
}

[a, b, c] = lengthTrackingWithOffset;
if (a !== 2 || b !== 3 || c !== undefined) {
  throw new Error("lengthTrackingWithOffset");
}
