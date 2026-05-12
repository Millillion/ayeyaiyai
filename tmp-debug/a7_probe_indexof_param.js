var o = { aa: 1, ba: 2, ca: 3 };
function check(hash_map, charactr) {
  for (var key in hash_map) {
    console.log(key);
    console.log(key.indexOf(charactr));
  }
}
check(o, "b");
