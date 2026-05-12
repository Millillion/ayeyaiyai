var o = { aa: 1, ba: 2, ca: 3 };
var target = o;
function erasator_T_1000(hash_map, charactr) {
  for (var key in hash_map) {
    if (key.indexOf(charactr) === 0) {
      delete hash_map[key];
    }
  }
}
erasator_T_1000(o, "b");
if ("ba" in target) {
  console.log("present");
} else {
  console.log("missing");
}
