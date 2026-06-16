var value = 1;
var floatValues = new Array(1076);
for (var power = 0; power <= 1075; power++) {
  floatValues[power] = value;
  value = value * 0.5;
}

console.log("v1075", floatValues[1075]);
console.log("v1074", floatValues[1074]);
console.log("min-lit", 4.9406564584124654417656879286822e-324);

if (floatValues[1075] !== 0) {
  console.log("fail-v1075");
}

if (floatValues[1074] !== 4.9406564584124654417656879286822e-324) {
  console.log("fail-v1074", floatValues[1074]);
}

for (var index = 1074; index > 0; index--) {
  if (floatValues[index] === 0) {
    console.log("fail-zero", index, floatValues[index]);
    break;
  }
  if (floatValues[index - 1] !== (floatValues[index] * 2)) {
    console.log(
      "fail-double",
      index,
      floatValues[index - 1],
      floatValues[index] * 2
    );
    break;
  }
}

console.log(
  "max-lt-inf",
  1.797693134862315708145274237317e+308 < Infinity,
  1.797693134862315708145274237317e+308
);
console.log(
  "overflow-eq-inf",
  1.797693134862315808e+308 === +Infinity,
  1.797693134862315808e+308
);
