Object.defineProperty(Object.prototype, "x", { get: function () { return this; } });

console.log((5).x == 0, (5).x == 5, (5).x === 5, typeof (5).x);
