var C = class {
  constructor() {
    throw new Error("constructor ran");
  }
};

try {
  var c = new C();
  throw new Error("constructor did not run");
} catch (error) {
  if (error.message !== "constructor ran") {
    throw error;
  }
}

