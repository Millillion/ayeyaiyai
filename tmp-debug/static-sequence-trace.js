var sequence = [];

class C {
  static x = (console.log('field1 before'), sequence.push('first field'));
  static {
    console.log('block1 before');
    sequence.push('first block');
  }
  static x = (console.log('field2 before'), sequence.push('second field'));
  static {
    console.log('block2 before');
    sequence.push('second block');
  }
}

console.log('done');
