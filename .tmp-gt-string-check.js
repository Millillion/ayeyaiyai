console.log("1");
if (("xy" > "xx") !== true) { throw new Error("1"); }
console.log("2");
if (("xx" > "xy") !== false) { throw new Error("2"); }
console.log("3");
if (("y" > "x") !== true) { throw new Error("3"); }
console.log("4");
if (("aba" > "aab") !== true) { throw new Error("4"); }
console.log("5");
if (("\u0061\u0061\u0061\u0061" > "\u0061\u0061\u0061\u0062") !== false) { throw new Error("5"); }
console.log("6");
if (("a\u0000b" > "a\u0000a") !== true) { throw new Error("6"); }
console.log("7");
if (("aa" > "aB") !== true) { throw new Error("7"); }
console.log("8");
if (("\u{10000}" > "\uD7FF") !== true) { throw new Error("8"); }
console.log("9");
if (("\uDC00" > "\uD800") !== true) { throw new Error("9"); }
console.log("10");
if (("\u{10000}" > "\uFFFF") !== false) { throw new Error("10"); }
console.log("11");
if (("\u{12345}" > "\u{10000}") !== true) { throw new Error("11"); }
console.log("ok");
