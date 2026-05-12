let nextCount = 0;
let returnCount = 0;
function Test262Error() {}
let x;
let iterator = {
  next() {
    nextCount += 1;
    console.log(nextCount);
    return { done: nextCount > 10 };
  },
  return() {
    returnCount += 1;
    throw new Test262Error();
  }
};
let iterable = {
  [Symbol.iterator]() {
    return iterator;
  }
};

async function * fn() {
  for await ([ x , ] of [iterable]) {
  }
}

let iter = fn();
console.log(nextCount);
iter.next().then(() => print(999), () => {
  console.log(nextCount);
  console.log(returnCount);
});
