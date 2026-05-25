(function(a) {
  let setCalls = 0;
  Object.defineProperty(arguments, "0", {
    set(_v) { setCalls += 1; },
    enumerable: true,
    configurable: true,
  });

  arguments[0] = "foo";
  console.log("after-set", setCalls, a, arguments[0], arguments[0] === undefined);

  Object.defineProperty(arguments, "1", {
    get: () => "bar",
    enumerable: true,
    configurable: true,
  });

  console.log("after-get", arguments[1], arguments[1] === "bar");
})(0);
