var __JEDI = "jedi";

function __FUNC() {
    function __GUNC() {
        return arguments[0];
    }

    return __GUNC;
}

var f = __FUNC();
console.log(typeof f);
console.log(f(__JEDI));
console.log(f("x"));
console.log(__FUNC()(__JEDI));
