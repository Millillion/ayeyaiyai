var desc = Object.getOwnPropertyDescriptor(x => {}, "name");

if (desc === undefined) {
  throw new Error("missing");
}

if (desc.value !== "") {
  throw new Error("wrong value");
}

if (desc.writable !== false) {
  throw new Error("wrong writable");
}

if (desc.enumerable !== false) {
  throw new Error("wrong enumerable");
}

if (desc.configurable !== true) {
  throw new Error("wrong configurable");
}
