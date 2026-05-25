var count = 0;
var knock = function () {
  count++;
};
console.log("before", count);
knock();
console.log("after direct", count);
this["knock"]();
console.log("after member", count);
