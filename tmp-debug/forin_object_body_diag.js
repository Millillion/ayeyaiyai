var __str = "", hash;
for (var ind in (hash = { 2: "b", 1: "a", 4: "d", 3: "c" })) {
  console.log(ind);
  console.log(hash[ind]);
  __str += hash[ind];
  console.log(__str);
}
console.log(__str);
