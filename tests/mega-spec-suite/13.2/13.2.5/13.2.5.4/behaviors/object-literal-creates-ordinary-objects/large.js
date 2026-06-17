// behavior: object-literal-creates-ordinary-objects
// expected: pass
// goal: script
// size: large
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

var emptyA10 = makeEmptyObject();
var emptyB10 = makeEmptyObject();
check(emptyA10 !== emptyB10, "fresh empty object 10");
check(Object.getPrototypeOf(emptyA10) === Object.prototype, "empty object prototype 10");
emptyA10.written = 140;
check(emptyB10.written === undefined, "fresh empty isolated 10");
var filledA10 = makeObjectWithValue(140);
var filledB10 = makeObjectWithValue(140);
check(filledA10 !== filledB10, "fresh filled object 10");
check(filledA10.nested !== filledB10.nested, "fresh nested object 10");
check(Object.getPrototypeOf(filledA10) === Object.prototype, "filled object prototype 10");
check(filledA10.value === 140 && filledA10.nested.inner === 140 + 1, "filled object values 10");

var emptyA11 = makeEmptyObject();
var emptyB11 = makeEmptyObject();
check(emptyA11 !== emptyB11, "fresh empty object 11");
check(Object.getPrototypeOf(emptyA11) === Object.prototype, "empty object prototype 11");
emptyA11.written = 151;
check(emptyB11.written === undefined, "fresh empty isolated 11");
var filledA11 = makeObjectWithValue(151);
var filledB11 = makeObjectWithValue(151);
check(filledA11 !== filledB11, "fresh filled object 11");
check(filledA11.nested !== filledB11.nested, "fresh nested object 11");
check(Object.getPrototypeOf(filledA11) === Object.prototype, "filled object prototype 11");
check(filledA11.value === 151 && filledA11.nested.inner === 151 + 1, "filled object values 11");

var emptyA12 = makeEmptyObject();
var emptyB12 = makeEmptyObject();
check(emptyA12 !== emptyB12, "fresh empty object 12");
check(Object.getPrototypeOf(emptyA12) === Object.prototype, "empty object prototype 12");
emptyA12.written = 162;
check(emptyB12.written === undefined, "fresh empty isolated 12");
var filledA12 = makeObjectWithValue(162);
var filledB12 = makeObjectWithValue(162);
check(filledA12 !== filledB12, "fresh filled object 12");
check(filledA12.nested !== filledB12.nested, "fresh nested object 12");
check(Object.getPrototypeOf(filledA12) === Object.prototype, "filled object prototype 12");
check(filledA12.value === 162 && filledA12.nested.inner === 162 + 1, "filled object values 12");

var emptyA13 = makeEmptyObject();
var emptyB13 = makeEmptyObject();
check(emptyA13 !== emptyB13, "fresh empty object 13");
check(Object.getPrototypeOf(emptyA13) === Object.prototype, "empty object prototype 13");
emptyA13.written = 173;
check(emptyB13.written === undefined, "fresh empty isolated 13");
var filledA13 = makeObjectWithValue(173);
var filledB13 = makeObjectWithValue(173);
check(filledA13 !== filledB13, "fresh filled object 13");
check(filledA13.nested !== filledB13.nested, "fresh nested object 13");
check(Object.getPrototypeOf(filledA13) === Object.prototype, "filled object prototype 13");
check(filledA13.value === 173 && filledA13.nested.inner === 173 + 1, "filled object values 13");

var emptyA14 = makeEmptyObject();
var emptyB14 = makeEmptyObject();
check(emptyA14 !== emptyB14, "fresh empty object 14");
check(Object.getPrototypeOf(emptyA14) === Object.prototype, "empty object prototype 14");
emptyA14.written = 184;
check(emptyB14.written === undefined, "fresh empty isolated 14");
var filledA14 = makeObjectWithValue(184);
var filledB14 = makeObjectWithValue(184);
check(filledA14 !== filledB14, "fresh filled object 14");
check(filledA14.nested !== filledB14.nested, "fresh nested object 14");
check(Object.getPrototypeOf(filledA14) === Object.prototype, "filled object prototype 14");
check(filledA14.value === 184 && filledA14.nested.inner === 184 + 1, "filled object values 14");

var emptyA15 = makeEmptyObject();
var emptyB15 = makeEmptyObject();
check(emptyA15 !== emptyB15, "fresh empty object 15");
check(Object.getPrototypeOf(emptyA15) === Object.prototype, "empty object prototype 15");
emptyA15.written = 195;
check(emptyB15.written === undefined, "fresh empty isolated 15");
var filledA15 = makeObjectWithValue(195);
var filledB15 = makeObjectWithValue(195);
check(filledA15 !== filledB15, "fresh filled object 15");
check(filledA15.nested !== filledB15.nested, "fresh nested object 15");
check(Object.getPrototypeOf(filledA15) === Object.prototype, "filled object prototype 15");
check(filledA15.value === 195 && filledA15.nested.inner === 195 + 1, "filled object values 15");

var emptyA16 = makeEmptyObject();
var emptyB16 = makeEmptyObject();
check(emptyA16 !== emptyB16, "fresh empty object 16");
check(Object.getPrototypeOf(emptyA16) === Object.prototype, "empty object prototype 16");
emptyA16.written = 206;
check(emptyB16.written === undefined, "fresh empty isolated 16");
var filledA16 = makeObjectWithValue(206);
var filledB16 = makeObjectWithValue(206);
check(filledA16 !== filledB16, "fresh filled object 16");
check(filledA16.nested !== filledB16.nested, "fresh nested object 16");
check(Object.getPrototypeOf(filledA16) === Object.prototype, "filled object prototype 16");
check(filledA16.value === 206 && filledA16.nested.inner === 206 + 1, "filled object values 16");

var emptyA17 = makeEmptyObject();
var emptyB17 = makeEmptyObject();
check(emptyA17 !== emptyB17, "fresh empty object 17");
check(Object.getPrototypeOf(emptyA17) === Object.prototype, "empty object prototype 17");
emptyA17.written = 217;
check(emptyB17.written === undefined, "fresh empty isolated 17");
var filledA17 = makeObjectWithValue(217);
var filledB17 = makeObjectWithValue(217);
check(filledA17 !== filledB17, "fresh filled object 17");
check(filledA17.nested !== filledB17.nested, "fresh nested object 17");
check(Object.getPrototypeOf(filledA17) === Object.prototype, "filled object prototype 17");
check(filledA17.value === 217 && filledA17.nested.inner === 217 + 1, "filled object values 17");

var emptyA18 = makeEmptyObject();
var emptyB18 = makeEmptyObject();
check(emptyA18 !== emptyB18, "fresh empty object 18");
check(Object.getPrototypeOf(emptyA18) === Object.prototype, "empty object prototype 18");
emptyA18.written = 228;
check(emptyB18.written === undefined, "fresh empty isolated 18");
var filledA18 = makeObjectWithValue(228);
var filledB18 = makeObjectWithValue(228);
check(filledA18 !== filledB18, "fresh filled object 18");
check(filledA18.nested !== filledB18.nested, "fresh nested object 18");
check(Object.getPrototypeOf(filledA18) === Object.prototype, "filled object prototype 18");
check(filledA18.value === 228 && filledA18.nested.inner === 228 + 1, "filled object values 18");

var emptyA19 = makeEmptyObject();
var emptyB19 = makeEmptyObject();
check(emptyA19 !== emptyB19, "fresh empty object 19");
check(Object.getPrototypeOf(emptyA19) === Object.prototype, "empty object prototype 19");
emptyA19.written = 239;
check(emptyB19.written === undefined, "fresh empty isolated 19");
var filledA19 = makeObjectWithValue(239);
var filledB19 = makeObjectWithValue(239);
check(filledA19 !== filledB19, "fresh filled object 19");
check(filledA19.nested !== filledB19.nested, "fresh nested object 19");
check(Object.getPrototypeOf(filledA19) === Object.prototype, "filled object prototype 19");
check(filledA19.value === 239 && filledA19.nested.inner === 239 + 1, "filled object values 19");

var emptyA20 = makeEmptyObject();
var emptyB20 = makeEmptyObject();
check(emptyA20 !== emptyB20, "fresh empty object 20");
check(Object.getPrototypeOf(emptyA20) === Object.prototype, "empty object prototype 20");
emptyA20.written = 250;
check(emptyB20.written === undefined, "fresh empty isolated 20");
var filledA20 = makeObjectWithValue(250);
var filledB20 = makeObjectWithValue(250);
check(filledA20 !== filledB20, "fresh filled object 20");
check(filledA20.nested !== filledB20.nested, "fresh nested object 20");
check(Object.getPrototypeOf(filledA20) === Object.prototype, "filled object prototype 20");
check(filledA20.value === 250 && filledA20.nested.inner === 250 + 1, "filled object values 20");

var emptyA21 = makeEmptyObject();
var emptyB21 = makeEmptyObject();
check(emptyA21 !== emptyB21, "fresh empty object 21");
check(Object.getPrototypeOf(emptyA21) === Object.prototype, "empty object prototype 21");
emptyA21.written = 261;
check(emptyB21.written === undefined, "fresh empty isolated 21");
var filledA21 = makeObjectWithValue(261);
var filledB21 = makeObjectWithValue(261);
check(filledA21 !== filledB21, "fresh filled object 21");
check(filledA21.nested !== filledB21.nested, "fresh nested object 21");
check(Object.getPrototypeOf(filledA21) === Object.prototype, "filled object prototype 21");
check(filledA21.value === 261 && filledA21.nested.inner === 261 + 1, "filled object values 21");

var emptyA22 = makeEmptyObject();
var emptyB22 = makeEmptyObject();
check(emptyA22 !== emptyB22, "fresh empty object 22");
check(Object.getPrototypeOf(emptyA22) === Object.prototype, "empty object prototype 22");
emptyA22.written = 272;
check(emptyB22.written === undefined, "fresh empty isolated 22");
var filledA22 = makeObjectWithValue(272);
var filledB22 = makeObjectWithValue(272);
check(filledA22 !== filledB22, "fresh filled object 22");
check(filledA22.nested !== filledB22.nested, "fresh nested object 22");
check(Object.getPrototypeOf(filledA22) === Object.prototype, "filled object prototype 22");
check(filledA22.value === 272 && filledA22.nested.inner === 272 + 1, "filled object values 22");

var emptyA23 = makeEmptyObject();
var emptyB23 = makeEmptyObject();
check(emptyA23 !== emptyB23, "fresh empty object 23");
check(Object.getPrototypeOf(emptyA23) === Object.prototype, "empty object prototype 23");
emptyA23.written = 283;
check(emptyB23.written === undefined, "fresh empty isolated 23");
var filledA23 = makeObjectWithValue(283);
var filledB23 = makeObjectWithValue(283);
check(filledA23 !== filledB23, "fresh filled object 23");
check(filledA23.nested !== filledB23.nested, "fresh nested object 23");
check(Object.getPrototypeOf(filledA23) === Object.prototype, "filled object prototype 23");
check(filledA23.value === 283 && filledA23.nested.inner === 283 + 1, "filled object values 23");

var emptyA24 = makeEmptyObject();
var emptyB24 = makeEmptyObject();
check(emptyA24 !== emptyB24, "fresh empty object 24");
check(Object.getPrototypeOf(emptyA24) === Object.prototype, "empty object prototype 24");
emptyA24.written = 294;
check(emptyB24.written === undefined, "fresh empty isolated 24");
var filledA24 = makeObjectWithValue(294);
var filledB24 = makeObjectWithValue(294);
check(filledA24 !== filledB24, "fresh filled object 24");
check(filledA24.nested !== filledB24.nested, "fresh nested object 24");
check(Object.getPrototypeOf(filledA24) === Object.prototype, "filled object prototype 24");
check(filledA24.value === 294 && filledA24.nested.inner === 294 + 1, "filled object values 24");

var emptyA25 = makeEmptyObject();
var emptyB25 = makeEmptyObject();
check(emptyA25 !== emptyB25, "fresh empty object 25");
check(Object.getPrototypeOf(emptyA25) === Object.prototype, "empty object prototype 25");
emptyA25.written = 305;
check(emptyB25.written === undefined, "fresh empty isolated 25");
var filledA25 = makeObjectWithValue(305);
var filledB25 = makeObjectWithValue(305);
check(filledA25 !== filledB25, "fresh filled object 25");
check(filledA25.nested !== filledB25.nested, "fresh nested object 25");
check(Object.getPrototypeOf(filledA25) === Object.prototype, "filled object prototype 25");
check(filledA25.value === 305 && filledA25.nested.inner === 305 + 1, "filled object values 25");

var emptyA26 = makeEmptyObject();
var emptyB26 = makeEmptyObject();
check(emptyA26 !== emptyB26, "fresh empty object 26");
check(Object.getPrototypeOf(emptyA26) === Object.prototype, "empty object prototype 26");
emptyA26.written = 316;
check(emptyB26.written === undefined, "fresh empty isolated 26");
var filledA26 = makeObjectWithValue(316);
var filledB26 = makeObjectWithValue(316);
check(filledA26 !== filledB26, "fresh filled object 26");
check(filledA26.nested !== filledB26.nested, "fresh nested object 26");
check(Object.getPrototypeOf(filledA26) === Object.prototype, "filled object prototype 26");
check(filledA26.value === 316 && filledA26.nested.inner === 316 + 1, "filled object values 26");

var emptyA27 = makeEmptyObject();
var emptyB27 = makeEmptyObject();
check(emptyA27 !== emptyB27, "fresh empty object 27");
check(Object.getPrototypeOf(emptyA27) === Object.prototype, "empty object prototype 27");
emptyA27.written = 327;
check(emptyB27.written === undefined, "fresh empty isolated 27");
var filledA27 = makeObjectWithValue(327);
var filledB27 = makeObjectWithValue(327);
check(filledA27 !== filledB27, "fresh filled object 27");
check(filledA27.nested !== filledB27.nested, "fresh nested object 27");
check(Object.getPrototypeOf(filledA27) === Object.prototype, "filled object prototype 27");
check(filledA27.value === 327 && filledA27.nested.inner === 327 + 1, "filled object values 27");

var emptyA28 = makeEmptyObject();
var emptyB28 = makeEmptyObject();
check(emptyA28 !== emptyB28, "fresh empty object 28");
check(Object.getPrototypeOf(emptyA28) === Object.prototype, "empty object prototype 28");
emptyA28.written = 338;
check(emptyB28.written === undefined, "fresh empty isolated 28");
var filledA28 = makeObjectWithValue(338);
var filledB28 = makeObjectWithValue(338);
check(filledA28 !== filledB28, "fresh filled object 28");
check(filledA28.nested !== filledB28.nested, "fresh nested object 28");
check(Object.getPrototypeOf(filledA28) === Object.prototype, "filled object prototype 28");
check(filledA28.value === 338 && filledA28.nested.inner === 338 + 1, "filled object values 28");

var emptyA29 = makeEmptyObject();
var emptyB29 = makeEmptyObject();
check(emptyA29 !== emptyB29, "fresh empty object 29");
check(Object.getPrototypeOf(emptyA29) === Object.prototype, "empty object prototype 29");
emptyA29.written = 349;
check(emptyB29.written === undefined, "fresh empty isolated 29");
var filledA29 = makeObjectWithValue(349);
var filledB29 = makeObjectWithValue(349);
check(filledA29 !== filledB29, "fresh filled object 29");
check(filledA29.nested !== filledB29.nested, "fresh nested object 29");
check(Object.getPrototypeOf(filledA29) === Object.prototype, "filled object prototype 29");
check(filledA29.value === 349 && filledA29.nested.inner === 349 + 1, "filled object values 29");

var emptyA30 = makeEmptyObject();
var emptyB30 = makeEmptyObject();
check(emptyA30 !== emptyB30, "fresh empty object 30");
check(Object.getPrototypeOf(emptyA30) === Object.prototype, "empty object prototype 30");
emptyA30.written = 360;
check(emptyB30.written === undefined, "fresh empty isolated 30");
var filledA30 = makeObjectWithValue(360);
var filledB30 = makeObjectWithValue(360);
check(filledA30 !== filledB30, "fresh filled object 30");
check(filledA30.nested !== filledB30.nested, "fresh nested object 30");
check(Object.getPrototypeOf(filledA30) === Object.prototype, "filled object prototype 30");
check(filledA30.value === 360 && filledA30.nested.inner === 360 + 1, "filled object values 30");

var emptyA31 = makeEmptyObject();
var emptyB31 = makeEmptyObject();
check(emptyA31 !== emptyB31, "fresh empty object 31");
check(Object.getPrototypeOf(emptyA31) === Object.prototype, "empty object prototype 31");
emptyA31.written = 371;
check(emptyB31.written === undefined, "fresh empty isolated 31");
var filledA31 = makeObjectWithValue(371);
var filledB31 = makeObjectWithValue(371);
check(filledA31 !== filledB31, "fresh filled object 31");
check(filledA31.nested !== filledB31.nested, "fresh nested object 31");
check(Object.getPrototypeOf(filledA31) === Object.prototype, "filled object prototype 31");
check(filledA31.value === 371 && filledA31.nested.inner === 371 + 1, "filled object values 31");

var emptyA32 = makeEmptyObject();
var emptyB32 = makeEmptyObject();
check(emptyA32 !== emptyB32, "fresh empty object 32");
check(Object.getPrototypeOf(emptyA32) === Object.prototype, "empty object prototype 32");
emptyA32.written = 382;
check(emptyB32.written === undefined, "fresh empty isolated 32");
var filledA32 = makeObjectWithValue(382);
var filledB32 = makeObjectWithValue(382);
check(filledA32 !== filledB32, "fresh filled object 32");
check(filledA32.nested !== filledB32.nested, "fresh nested object 32");
check(Object.getPrototypeOf(filledA32) === Object.prototype, "filled object prototype 32");
check(filledA32.value === 382 && filledA32.nested.inner === 382 + 1, "filled object values 32");

var emptyA33 = makeEmptyObject();
var emptyB33 = makeEmptyObject();
check(emptyA33 !== emptyB33, "fresh empty object 33");
check(Object.getPrototypeOf(emptyA33) === Object.prototype, "empty object prototype 33");
emptyA33.written = 393;
check(emptyB33.written === undefined, "fresh empty isolated 33");
var filledA33 = makeObjectWithValue(393);
var filledB33 = makeObjectWithValue(393);
check(filledA33 !== filledB33, "fresh filled object 33");
check(filledA33.nested !== filledB33.nested, "fresh nested object 33");
check(Object.getPrototypeOf(filledA33) === Object.prototype, "filled object prototype 33");
check(filledA33.value === 393 && filledA33.nested.inner === 393 + 1, "filled object values 33");

var emptyA34 = makeEmptyObject();
var emptyB34 = makeEmptyObject();
check(emptyA34 !== emptyB34, "fresh empty object 34");
check(Object.getPrototypeOf(emptyA34) === Object.prototype, "empty object prototype 34");
emptyA34.written = 404;
check(emptyB34.written === undefined, "fresh empty isolated 34");
var filledA34 = makeObjectWithValue(404);
var filledB34 = makeObjectWithValue(404);
check(filledA34 !== filledB34, "fresh filled object 34");
check(filledA34.nested !== filledB34.nested, "fresh nested object 34");
check(Object.getPrototypeOf(filledA34) === Object.prototype, "filled object prototype 34");
check(filledA34.value === 404 && filledA34.nested.inner === 404 + 1, "filled object values 34");

var emptyA35 = makeEmptyObject();
var emptyB35 = makeEmptyObject();
check(emptyA35 !== emptyB35, "fresh empty object 35");
check(Object.getPrototypeOf(emptyA35) === Object.prototype, "empty object prototype 35");
emptyA35.written = 415;
check(emptyB35.written === undefined, "fresh empty isolated 35");
var filledA35 = makeObjectWithValue(415);
var filledB35 = makeObjectWithValue(415);
check(filledA35 !== filledB35, "fresh filled object 35");
check(filledA35.nested !== filledB35.nested, "fresh nested object 35");
check(Object.getPrototypeOf(filledA35) === Object.prototype, "filled object prototype 35");
check(filledA35.value === 415 && filledA35.nested.inner === 415 + 1, "filled object values 35");

var emptyA36 = makeEmptyObject();
var emptyB36 = makeEmptyObject();
check(emptyA36 !== emptyB36, "fresh empty object 36");
check(Object.getPrototypeOf(emptyA36) === Object.prototype, "empty object prototype 36");
emptyA36.written = 426;
check(emptyB36.written === undefined, "fresh empty isolated 36");
var filledA36 = makeObjectWithValue(426);
var filledB36 = makeObjectWithValue(426);
check(filledA36 !== filledB36, "fresh filled object 36");
check(filledA36.nested !== filledB36.nested, "fresh nested object 36");
check(Object.getPrototypeOf(filledA36) === Object.prototype, "filled object prototype 36");
check(filledA36.value === 426 && filledA36.nested.inner === 426 + 1, "filled object values 36");

var emptyA37 = makeEmptyObject();
var emptyB37 = makeEmptyObject();
check(emptyA37 !== emptyB37, "fresh empty object 37");
check(Object.getPrototypeOf(emptyA37) === Object.prototype, "empty object prototype 37");
emptyA37.written = 437;
check(emptyB37.written === undefined, "fresh empty isolated 37");
var filledA37 = makeObjectWithValue(437);
var filledB37 = makeObjectWithValue(437);
check(filledA37 !== filledB37, "fresh filled object 37");
check(filledA37.nested !== filledB37.nested, "fresh nested object 37");
check(Object.getPrototypeOf(filledA37) === Object.prototype, "filled object prototype 37");
check(filledA37.value === 437 && filledA37.nested.inner === 437 + 1, "filled object values 37");

var emptyA38 = makeEmptyObject();
var emptyB38 = makeEmptyObject();
check(emptyA38 !== emptyB38, "fresh empty object 38");
check(Object.getPrototypeOf(emptyA38) === Object.prototype, "empty object prototype 38");
emptyA38.written = 448;
check(emptyB38.written === undefined, "fresh empty isolated 38");
var filledA38 = makeObjectWithValue(448);
var filledB38 = makeObjectWithValue(448);
check(filledA38 !== filledB38, "fresh filled object 38");
check(filledA38.nested !== filledB38.nested, "fresh nested object 38");
check(Object.getPrototypeOf(filledA38) === Object.prototype, "filled object prototype 38");
check(filledA38.value === 448 && filledA38.nested.inner === 448 + 1, "filled object values 38");

var emptyA39 = makeEmptyObject();
var emptyB39 = makeEmptyObject();
check(emptyA39 !== emptyB39, "fresh empty object 39");
check(Object.getPrototypeOf(emptyA39) === Object.prototype, "empty object prototype 39");
emptyA39.written = 459;
check(emptyB39.written === undefined, "fresh empty isolated 39");
var filledA39 = makeObjectWithValue(459);
var filledB39 = makeObjectWithValue(459);
check(filledA39 !== filledB39, "fresh filled object 39");
check(filledA39.nested !== filledB39.nested, "fresh nested object 39");
check(Object.getPrototypeOf(filledA39) === Object.prototype, "filled object prototype 39");
check(filledA39.value === 459 && filledA39.nested.inner === 459 + 1, "filled object values 39");

var emptyA40 = makeEmptyObject();
var emptyB40 = makeEmptyObject();
check(emptyA40 !== emptyB40, "fresh empty object 40");
check(Object.getPrototypeOf(emptyA40) === Object.prototype, "empty object prototype 40");
emptyA40.written = 470;
check(emptyB40.written === undefined, "fresh empty isolated 40");
var filledA40 = makeObjectWithValue(470);
var filledB40 = makeObjectWithValue(470);
check(filledA40 !== filledB40, "fresh filled object 40");
check(filledA40.nested !== filledB40.nested, "fresh nested object 40");
check(Object.getPrototypeOf(filledA40) === Object.prototype, "filled object prototype 40");
check(filledA40.value === 470 && filledA40.nested.inner === 470 + 1, "filled object values 40");

var emptyA41 = makeEmptyObject();
var emptyB41 = makeEmptyObject();
check(emptyA41 !== emptyB41, "fresh empty object 41");
check(Object.getPrototypeOf(emptyA41) === Object.prototype, "empty object prototype 41");
emptyA41.written = 481;
check(emptyB41.written === undefined, "fresh empty isolated 41");
var filledA41 = makeObjectWithValue(481);
var filledB41 = makeObjectWithValue(481);
check(filledA41 !== filledB41, "fresh filled object 41");
check(filledA41.nested !== filledB41.nested, "fresh nested object 41");
check(Object.getPrototypeOf(filledA41) === Object.prototype, "filled object prototype 41");
check(filledA41.value === 481 && filledA41.nested.inner === 481 + 1, "filled object values 41");

var emptyA42 = makeEmptyObject();
var emptyB42 = makeEmptyObject();
check(emptyA42 !== emptyB42, "fresh empty object 42");
check(Object.getPrototypeOf(emptyA42) === Object.prototype, "empty object prototype 42");
emptyA42.written = 492;
check(emptyB42.written === undefined, "fresh empty isolated 42");
var filledA42 = makeObjectWithValue(492);
var filledB42 = makeObjectWithValue(492);
check(filledA42 !== filledB42, "fresh filled object 42");
check(filledA42.nested !== filledB42.nested, "fresh nested object 42");
check(Object.getPrototypeOf(filledA42) === Object.prototype, "filled object prototype 42");
check(filledA42.value === 492 && filledA42.nested.inner === 492 + 1, "filled object values 42");

var emptyA43 = makeEmptyObject();
var emptyB43 = makeEmptyObject();
check(emptyA43 !== emptyB43, "fresh empty object 43");
check(Object.getPrototypeOf(emptyA43) === Object.prototype, "empty object prototype 43");
emptyA43.written = 503;
check(emptyB43.written === undefined, "fresh empty isolated 43");
var filledA43 = makeObjectWithValue(503);
var filledB43 = makeObjectWithValue(503);
check(filledA43 !== filledB43, "fresh filled object 43");
check(filledA43.nested !== filledB43.nested, "fresh nested object 43");
check(Object.getPrototypeOf(filledA43) === Object.prototype, "filled object prototype 43");
check(filledA43.value === 503 && filledA43.nested.inner === 503 + 1, "filled object values 43");

var emptyA44 = makeEmptyObject();
var emptyB44 = makeEmptyObject();
check(emptyA44 !== emptyB44, "fresh empty object 44");
check(Object.getPrototypeOf(emptyA44) === Object.prototype, "empty object prototype 44");
emptyA44.written = 514;
check(emptyB44.written === undefined, "fresh empty isolated 44");
var filledA44 = makeObjectWithValue(514);
var filledB44 = makeObjectWithValue(514);
check(filledA44 !== filledB44, "fresh filled object 44");
check(filledA44.nested !== filledB44.nested, "fresh nested object 44");
check(Object.getPrototypeOf(filledA44) === Object.prototype, "filled object prototype 44");
check(filledA44.value === 514 && filledA44.nested.inner === 514 + 1, "filled object values 44");

check(score > 0, "ordinary object score");
