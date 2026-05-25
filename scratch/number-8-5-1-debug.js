var value = 1;
var floatValues = new Array(1076);
for (var power = 0; power <= 1075; power++) {
  floatValues[power] = value;
  value = value * 0.5;
}

console.log(floatValues[1075] === 0);
console.log(floatValues[1074] === 4.9406564584124654417656879286822e-324);
console.log(floatValues[1074] === 0);
console.log(floatValues[1022]);
console.log(floatValues[1023]);
console.log(floatValues[1074]);
console.log(floatValues[1075]);
console.log(1.797693134862315708145274237317e+308 < Infinity);
console.log(1.797693134862315808e+308 === +Infinity);
