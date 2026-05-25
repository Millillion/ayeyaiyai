var items = new Array("one", "two", "three");
var itemsRef = items;
items = new Array("new", "array");
console.log(items == itemsRef);
console.log(items.length);
console.log(itemsRef.length);
