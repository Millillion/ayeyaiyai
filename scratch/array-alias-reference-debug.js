var items = new Array("one", "two", "three");
var itemsRef = items;
items.push("four");
console.log(itemsRef.length);

var items2 = new Array("one", "two", "three");
var itemsRef2 = items2;
items2[1] = "duo";
console.log(itemsRef2[1]);
