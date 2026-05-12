function CreateResizableArrayBuffer(byteLength, maxByteLength) {
  return new ArrayBuffer(byteLength, { maxByteLength: maxByteLength });
}

function TestIterationAndResize(iterable, rab, resizeAfter, newByteLength) {
  let values = [];
  let resized = false;
  console.log("helper-start");
  for (let value of iterable) {
    console.log("iter", values.length, value);
    values.push(Number(value));
    if (!resized && values.length == resizeAfter) {
      console.log("resize-before", newByteLength);
      rab.resize(newByteLength);
      console.log("resize-after");
      resized = true;
    }
  }
  console.log("helper-end", values.length, resized);
}

function CreateRab(bufferByteLength, ctor) {
  console.log("create-rab-start", bufferByteLength);
  const rab = CreateResizableArrayBuffer(bufferByteLength, 2 * bufferByteLength);
  console.log("construct-writer");
  let taWrite = new ctor(rab);
  console.log("fill-start", taWrite.length);
  for (let i = 0; i < bufferByteLength / ctor.BYTES_PER_ELEMENT; ++i) {
    taWrite[i] = i % 128;
  }
  console.log("create-rab-end");
  return rab;
}

const ctors = [Uint8Array, Int8Array, Uint16Array, Int16Array, Uint32Array, Int32Array, Float32Array, Float64Array, Uint8ClampedArray, BigUint64Array, BigInt64Array];
for (let ctor of ctors) {
const noElements = 10;
const offset = 2;
const bufferByteLength = noElements * ctor.BYTES_PER_ELEMENT;
const byteOffset = offset * ctor.BYTES_PER_ELEMENT;

let rab = CreateRab(bufferByteLength, ctor);
console.log("construct-view", ctor.BYTES_PER_ELEMENT);
const lengthTrackingTa = new ctor(rab);
TestIterationAndResize(lengthTrackingTa, rab, noElements, bufferByteLength * 2);

rab = CreateRab(bufferByteLength, ctor);
console.log("construct-view-offset", byteOffset);
const lengthTrackingTaWithOffset = new ctor(rab, byteOffset);
TestIterationAndResize(lengthTrackingTaWithOffset, rab, noElements - offset, bufferByteLength * 2);
}
