this.p1 = "alert";

__obj = {
    p1: 1,
    getRight: function () {
        return "right";
    },
};

getRight = function () {
    return "napravo";
};

try {
    (function () {
        with (__obj) {
            p1 = "w1";
            getRight = function () {
                return false;
            };
            throw p1;
        }
    })();
} catch (e) {
    resukt = p1;
    console.log(e);
}

console.log(p1);
console.log(getRight());
console.log(__obj.p1);
console.log(__obj.getRight());
console.log(resukt);
console.log(p1 === "alert");
console.log(getRight() === "napravo");
console.log(__obj.p1 === "w1");
console.log(__obj.getRight() === false);
console.log(resukt === "alert");

var resukt;
