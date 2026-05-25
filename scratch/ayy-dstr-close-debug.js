var initCount = 0;
var iterCount = 0;
var iter = function*() {
  console.log("generator-start");
  iterCount += 1;
}();

var callCount = 0;
var f;
f = function([[] = function() {
  initCount += 1;
  console.log("default", initCount, iterCount);
  return iter;
}()]) {
  console.log("body", initCount, iterCount);
  callCount = callCount + 1;
};

f([]);
console.log("after", initCount, iterCount, callCount);
