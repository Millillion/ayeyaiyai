var __obj, __key;
__obj = { aa: 1, ba: 2, ca: 3 };
for (__key in __obj) {
  erasator_T_1000(__obj, "b");
  console.log(__key);
}
function erasator_T_1000(hash_map, charactr) {
  for (var key in hash_map) {
    if (key.indexOf(charactr) === 0) {
      delete hash_map[key];
    }
  }
}
