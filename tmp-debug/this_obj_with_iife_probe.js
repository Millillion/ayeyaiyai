this.__obj = { p1: 1 };
(function () {
    with (__obj) {
        p1 = "w1";
    }
})();
console.log(__obj.p1 === "w1");
