var sequence = [];

class C {
  static x = sequence.push('first field');
  static {
    sequence.push('first block');
  }
  static x = sequence.push('second field');
  static {
    sequence.push('second block');
  }
}

console.log(sequence[0], sequence[1], sequence[2], sequence[3], C.x);
