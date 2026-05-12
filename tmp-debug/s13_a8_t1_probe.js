var __FRST = "one";
var __SCND = "two";
var __func = function(arg1, arg2) {
  this.first = arg1;
  __gunc.prop = arg2;
  return __gunc;
  function __gunc(arg) { return ++arg; }
};
var __instance = new __func(__FRST, __SCND);
console.log(__instance.first);
console.log(__instance.prop);
console.log(typeof __instance);
console.log(__instance(1));
