this.p1 = 1;
this.p2 = 2;
this.p3 = 3;
var result = "result";
var myObj = {
  p1: "a",
  p2: "b",
  p3: "c",
  value: "myObj_value",
  valueOf: function () { return "obj_valueOf"; },
  parseInt: function () { return "obj_parseInt"; },
  NaN: "obj_NaN",
  Infinity: "obj_Infinity",
  eval: function () { return "obj_eval"; },
  parseFloat: function () { return "obj_parseFloat"; },
  isNaN: function () { return "obj_isNaN"; },
  isFinite: function () { return "obj_isFinite"; }
};
var del;
var st_p1 = "p1";
var st_p2 = "p2";
var st_p3 = "p3";
var st_parseInt = "parseInt";
var st_NaN = "NaN";
var st_Infinity = "Infinity";
var st_eval = "eval";
var st_parseFloat = "parseFloat";
var st_isNaN = "isNaN";
var st_isFinite = "isFinite";

with (myObj) {
  var f = function () {
    return value;
    st_p1 = p1;
    st_p2 = p2;
    st_p3 = p3;
    st_parseInt = parseInt;
    st_NaN = NaN;
    st_Infinity = Infinity;
    st_eval = eval;
    st_parseFloat = parseFloat;
    st_isNaN = isNaN;
    st_isFinite = isFinite;
    p1 = "x1";
    this.p2 = "x2";
    del = delete p3;
    var p4 = "x4";
    p5 = "x5";
    var value = "value";
  };
}
console.log("before");
result = f();
var p4Missing = false;
try { p4; } catch (e) { p4Missing = true; }
var p5Missing = false;
try { p5; } catch (e) { p5Missing = true; }
var valueMissing = false;
try { value; } catch (e) { valueMissing = true; }
console.log("after", result, p1, p2, p3, myObj.p1, myObj.p2, myObj.p3, myObj.p4, myObj.p5, myObj.value);
console.log("st", st_p1, st_p2, st_p3, st_parseInt, st_NaN, st_Infinity, st_eval, st_parseFloat, st_isNaN, st_isFinite);
console.log("missing", p4Missing, p5Missing, valueMissing);
console.log("checks",
  result === undefined,
  p1 === 1,
  p2 === 2,
  p3 === 3,
  p4Missing,
  p5Missing,
  myObj.p1 === "a",
  myObj.p2 === "b",
  myObj.p3 === "c",
  myObj.p4 === undefined,
  myObj.p5 === undefined,
  st_parseInt === "parseInt",
  st_NaN === "NaN",
  st_Infinity === "Infinity",
  st_eval === "eval",
  st_parseFloat === "parseFloat",
  st_isNaN === "isNaN",
  st_isFinite === "isFinite",
  valueMissing,
  myObj.value === "myObj_value"
);
