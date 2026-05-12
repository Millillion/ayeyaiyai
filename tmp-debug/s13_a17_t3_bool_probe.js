p1 = "alert";
this.__obj = { p1: 1, getRight: function () { return "right"; } };
var getRight = function () { return "napravo"; };
resukt = (function () {
    with (__obj) {
        p1 = "w1";
        var getRight = function () { return false; };
        return p1;
    }
})();
console.log(p1 === "alert");
console.log(getRight() === "napravo");
console.log(__obj.p1 === "w1");
console.log(__obj.getRight() === false);
console.log(resukt === "w1");
var resukt;
