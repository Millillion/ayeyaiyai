var __FRST = "one";
var __SCND = "two";

var __func = function(arg1, arg2) {
    this.first = arg1;
    var __gunc = Function.call(this, "arg", "return ++arg;");
    __gunc.prop = arg2;
    return __gunc;
};

var __instance = new __func(__FRST, __SCND);

console.log(typeof __instance);
console.log(__instance.first);
console.log(__instance.prop);
console.log(__instance(1));
