var o = { aa: 1, ba: 2, ca: 3 };
function clear_all(hash_map) {
  for (var key in hash_map) {
    delete hash_map[key];
  }
}
clear_all(o);
console.log("done");
