var sequence = [];

class C {
  static x = sequence.push('first field');
}

console.log(sequence[0], C.x);
