var x = "MAX_VALUE";
var y = Number;
console.log("literal-right", "MAX_VALUE" in Number);
console.log("alias-left", x in Number);
console.log("alias-right", "MAX_VALUE" in y);
console.log("both-alias", x in y);
