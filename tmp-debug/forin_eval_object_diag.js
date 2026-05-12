var __str, __evaluated, hash;
__str = "";
__evaluated = eval("for(var ind in (hash={2:'b',1:'a',4:'d',3:'c'}))__str+=hash[ind]");
console.log(__evaluated);
console.log(__str);
console.log(__evaluated.indexOf("a"), __evaluated.indexOf("b"), __evaluated.indexOf("c"), __evaluated.indexOf("d"));
