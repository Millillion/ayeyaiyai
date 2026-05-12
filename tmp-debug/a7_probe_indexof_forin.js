var o = { aa: 1, ba: 2, ca: 3 };
for (var key in o) {
  console.log(key);
  console.log(key.indexOf("b"));
}
