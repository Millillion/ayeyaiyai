var a = 1;
var __obj = {a:2,__obj:{a:3}};

try {
    with (__obj) {
        with (__obj) {
            var __func = function(){return a;};
            throw 5;
        }
    }
} catch (e) {
    ;
}

result = __func();
print(result);
print(a);
print(__obj.a);
print(__obj.__obj.a);
print(typeof __func);
