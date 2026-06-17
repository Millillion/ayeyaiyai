// behavior: object-literal-creates-ordinary-objects
// expected: pass
// goal: script
// size: standard
// variant: script.sloppy

var score = 0;
function check(condition, label) {
if (!condition) {
throw label;
}
score = score + 1;
return true;
}

function makeEmptyObject() {
return {};
}
function makeObjectWithValue(value) {
return { value: value, nested: { inner: value + 1 } };
}

var emptyA0 = makeEmptyObject();
var emptyB0 = makeEmptyObject();
check(emptyA0 !== emptyB0, "fresh empty object 0");
check(Object.getPrototypeOf(emptyA0) === Object.prototype, "empty object prototype 0");
emptyA0.written = 30;
check(emptyB0.written === undefined, "fresh empty isolated 0");
var filledA0 = makeObjectWithValue(30);
var filledB0 = makeObjectWithValue(30);
check(filledA0 !== filledB0, "fresh filled object 0");
check(filledA0.nested !== filledB0.nested, "fresh nested object 0");
check(Object.getPrototypeOf(filledA0) === Object.prototype, "filled object prototype 0");
check(filledA0.value === 30 && filledA0.nested.inner === 30 + 1, "filled object values 0");

var emptyA1 = makeEmptyObject();
var emptyB1 = makeEmptyObject();
check(emptyA1 !== emptyB1, "fresh empty object 1");
check(Object.getPrototypeOf(emptyA1) === Object.prototype, "empty object prototype 1");
emptyA1.written = 41;
check(emptyB1.written === undefined, "fresh empty isolated 1");
var filledA1 = makeObjectWithValue(41);
var filledB1 = makeObjectWithValue(41);
check(filledA1 !== filledB1, "fresh filled object 1");
check(filledA1.nested !== filledB1.nested, "fresh nested object 1");
check(Object.getPrototypeOf(filledA1) === Object.prototype, "filled object prototype 1");
check(filledA1.value === 41 && filledA1.nested.inner === 41 + 1, "filled object values 1");

var emptyA2 = makeEmptyObject();
var emptyB2 = makeEmptyObject();
check(emptyA2 !== emptyB2, "fresh empty object 2");
check(Object.getPrototypeOf(emptyA2) === Object.prototype, "empty object prototype 2");
emptyA2.written = 52;
check(emptyB2.written === undefined, "fresh empty isolated 2");
var filledA2 = makeObjectWithValue(52);
var filledB2 = makeObjectWithValue(52);
check(filledA2 !== filledB2, "fresh filled object 2");
check(filledA2.nested !== filledB2.nested, "fresh nested object 2");
check(Object.getPrototypeOf(filledA2) === Object.prototype, "filled object prototype 2");
check(filledA2.value === 52 && filledA2.nested.inner === 52 + 1, "filled object values 2");

var emptyA3 = makeEmptyObject();
var emptyB3 = makeEmptyObject();
check(emptyA3 !== emptyB3, "fresh empty object 3");
check(Object.getPrototypeOf(emptyA3) === Object.prototype, "empty object prototype 3");
emptyA3.written = 63;
check(emptyB3.written === undefined, "fresh empty isolated 3");
var filledA3 = makeObjectWithValue(63);
var filledB3 = makeObjectWithValue(63);
check(filledA3 !== filledB3, "fresh filled object 3");
check(filledA3.nested !== filledB3.nested, "fresh nested object 3");
check(Object.getPrototypeOf(filledA3) === Object.prototype, "filled object prototype 3");
check(filledA3.value === 63 && filledA3.nested.inner === 63 + 1, "filled object values 3");

var emptyA4 = makeEmptyObject();
var emptyB4 = makeEmptyObject();
check(emptyA4 !== emptyB4, "fresh empty object 4");
check(Object.getPrototypeOf(emptyA4) === Object.prototype, "empty object prototype 4");
emptyA4.written = 74;
check(emptyB4.written === undefined, "fresh empty isolated 4");
var filledA4 = makeObjectWithValue(74);
var filledB4 = makeObjectWithValue(74);
check(filledA4 !== filledB4, "fresh filled object 4");
check(filledA4.nested !== filledB4.nested, "fresh nested object 4");
check(Object.getPrototypeOf(filledA4) === Object.prototype, "filled object prototype 4");
check(filledA4.value === 74 && filledA4.nested.inner === 74 + 1, "filled object values 4");

var emptyA5 = makeEmptyObject();
var emptyB5 = makeEmptyObject();
check(emptyA5 !== emptyB5, "fresh empty object 5");
check(Object.getPrototypeOf(emptyA5) === Object.prototype, "empty object prototype 5");
emptyA5.written = 85;
check(emptyB5.written === undefined, "fresh empty isolated 5");
var filledA5 = makeObjectWithValue(85);
var filledB5 = makeObjectWithValue(85);
check(filledA5 !== filledB5, "fresh filled object 5");
check(filledA5.nested !== filledB5.nested, "fresh nested object 5");
check(Object.getPrototypeOf(filledA5) === Object.prototype, "filled object prototype 5");
check(filledA5.value === 85 && filledA5.nested.inner === 85 + 1, "filled object values 5");

var emptyA6 = makeEmptyObject();
var emptyB6 = makeEmptyObject();
check(emptyA6 !== emptyB6, "fresh empty object 6");
check(Object.getPrototypeOf(emptyA6) === Object.prototype, "empty object prototype 6");
emptyA6.written = 96;
check(emptyB6.written === undefined, "fresh empty isolated 6");
var filledA6 = makeObjectWithValue(96);
var filledB6 = makeObjectWithValue(96);
check(filledA6 !== filledB6, "fresh filled object 6");
check(filledA6.nested !== filledB6.nested, "fresh nested object 6");
check(Object.getPrototypeOf(filledA6) === Object.prototype, "filled object prototype 6");
check(filledA6.value === 96 && filledA6.nested.inner === 96 + 1, "filled object values 6");

var emptyA7 = makeEmptyObject();
var emptyB7 = makeEmptyObject();
check(emptyA7 !== emptyB7, "fresh empty object 7");
check(Object.getPrototypeOf(emptyA7) === Object.prototype, "empty object prototype 7");
emptyA7.written = 107;
check(emptyB7.written === undefined, "fresh empty isolated 7");
var filledA7 = makeObjectWithValue(107);
var filledB7 = makeObjectWithValue(107);
check(filledA7 !== filledB7, "fresh filled object 7");
check(filledA7.nested !== filledB7.nested, "fresh nested object 7");
check(Object.getPrototypeOf(filledA7) === Object.prototype, "filled object prototype 7");
check(filledA7.value === 107 && filledA7.nested.inner === 107 + 1, "filled object values 7");

var emptyA8 = makeEmptyObject();
var emptyB8 = makeEmptyObject();
check(emptyA8 !== emptyB8, "fresh empty object 8");
check(Object.getPrototypeOf(emptyA8) === Object.prototype, "empty object prototype 8");
emptyA8.written = 118;
check(emptyB8.written === undefined, "fresh empty isolated 8");
var filledA8 = makeObjectWithValue(118);
var filledB8 = makeObjectWithValue(118);
check(filledA8 !== filledB8, "fresh filled object 8");
check(filledA8.nested !== filledB8.nested, "fresh nested object 8");
check(Object.getPrototypeOf(filledA8) === Object.prototype, "filled object prototype 8");
check(filledA8.value === 118 && filledA8.nested.inner === 118 + 1, "filled object values 8");

var emptyA9 = makeEmptyObject();
var emptyB9 = makeEmptyObject();
check(emptyA9 !== emptyB9, "fresh empty object 9");
check(Object.getPrototypeOf(emptyA9) === Object.prototype, "empty object prototype 9");
emptyA9.written = 129;
check(emptyB9.written === undefined, "fresh empty isolated 9");
var filledA9 = makeObjectWithValue(129);
var filledB9 = makeObjectWithValue(129);
check(filledA9 !== filledB9, "fresh filled object 9");
check(filledA9.nested !== filledB9.nested, "fresh nested object 9");
check(Object.getPrototypeOf(filledA9) === Object.prototype, "filled object prototype 9");
check(filledA9.value === 129 && filledA9.nested.inner === 129 + 1, "filled object values 9");

check(score > 0, "ordinary object score");
