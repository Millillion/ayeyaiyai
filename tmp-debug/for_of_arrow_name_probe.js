var arrow;
var counter = 0;

for ([arrow = () => {}] of [[]]) {
  console.log(arrow.name);
  counter += 1;
}

console.log(counter);
