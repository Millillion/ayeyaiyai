var array = [0, 'a', true, false, null, , undefined, NaN];
var i = 0;

for (var value of array.entries()) {
  console.log(i, value[0], value[1], value.length, array[i]);
  i++;
}

console.log("done", i);
