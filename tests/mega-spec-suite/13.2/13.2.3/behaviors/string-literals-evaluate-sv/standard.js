// behavior: string-literals-evaluate-sv
// expected: pass
// goal: script
// size: standard
// variant: script.sloppy

var score = 0;
function check(condition, label) {
if (!condition) {
throw label;
}
score = score + 1;
return true;
}
function sameString(value) { return value; }
var plainString0 = "literal-0";
check(plainString0 === 'literal-0', 'plain string 0');
check(plainString0.length === 9, 'plain length 0');
var singleString0 = 'single-0';
check(singleString0 === "single-0", 'single string 0');
var newlineString0 = "row\n0";
check(newlineString0.length === 5, 'newline length 0');
var hexEscapeString0 = "\x41-0";
check(hexEscapeString0 === 'A-0', 'hex escape string 0');
var unicodeEscapeString0 = "\u0041\u0042-0";
check(unicodeEscapeString0 === 'AB-0', 'unicode escape string 0');
var codePointString0 = "\u{1F600}-0";
check(codePointString0.length === 4, 'code point length 0');
var quoteString0 = "\"\'0";
check(quoteString0.length === 3, 'quote escape length 0');
var slashString0 = "path\\0";
check(slashString0 === 'path\\0', 'slash escape string 0');
var nullEscapeString0 = "\0x0";
check(nullEscapeString0.length === 3, 'null escape length 0');
var continuedString0 = "left\
right0";
check(continuedString0 === 'leftright0', 'line continuation string 0');
check(sameString(plainString0) === 'literal-0', 'string argument 0');
var plainString1 = "literal-1";
check(plainString1 === 'literal-1', 'plain string 1');
check(plainString1.length === 9, 'plain length 1');
var singleString1 = 'single-1';
check(singleString1 === "single-1", 'single string 1');
var newlineString1 = "row\n1";
check(newlineString1.length === 5, 'newline length 1');
var hexEscapeString1 = "\x41-1";
check(hexEscapeString1 === 'A-1', 'hex escape string 1');
var unicodeEscapeString1 = "\u0041\u0042-1";
check(unicodeEscapeString1 === 'AB-1', 'unicode escape string 1');
var codePointString1 = "\u{1F600}-1";
check(codePointString1.length === 4, 'code point length 1');
var quoteString1 = "\"\'1";
check(quoteString1.length === 3, 'quote escape length 1');
var slashString1 = "path\\1";
check(slashString1 === 'path\\1', 'slash escape string 1');
var nullEscapeString1 = "\0x1";
check(nullEscapeString1.length === 3, 'null escape length 1');
var continuedString1 = "left\
right1";
check(continuedString1 === 'leftright1', 'line continuation string 1');
check(sameString(plainString1) === 'literal-1', 'string argument 1');
var plainString2 = "literal-2";
check(plainString2 === 'literal-2', 'plain string 2');
check(plainString2.length === 9, 'plain length 2');
var singleString2 = 'single-2';
check(singleString2 === "single-2", 'single string 2');
var newlineString2 = "row\n2";
check(newlineString2.length === 5, 'newline length 2');
var hexEscapeString2 = "\x41-2";
check(hexEscapeString2 === 'A-2', 'hex escape string 2');
var unicodeEscapeString2 = "\u0041\u0042-2";
check(unicodeEscapeString2 === 'AB-2', 'unicode escape string 2');
var codePointString2 = "\u{1F600}-2";
check(codePointString2.length === 4, 'code point length 2');
var quoteString2 = "\"\'2";
check(quoteString2.length === 3, 'quote escape length 2');
var slashString2 = "path\\2";
check(slashString2 === 'path\\2', 'slash escape string 2');
var nullEscapeString2 = "\0x2";
check(nullEscapeString2.length === 3, 'null escape length 2');
var continuedString2 = "left\
right2";
check(continuedString2 === 'leftright2', 'line continuation string 2');
check(sameString(plainString2) === 'literal-2', 'string argument 2');
var plainString3 = "literal-3";
check(plainString3 === 'literal-3', 'plain string 3');
check(plainString3.length === 9, 'plain length 3');
var singleString3 = 'single-3';
check(singleString3 === "single-3", 'single string 3');
var newlineString3 = "row\n3";
check(newlineString3.length === 5, 'newline length 3');
var hexEscapeString3 = "\x41-3";
check(hexEscapeString3 === 'A-3', 'hex escape string 3');
var unicodeEscapeString3 = "\u0041\u0042-3";
check(unicodeEscapeString3 === 'AB-3', 'unicode escape string 3');
var codePointString3 = "\u{1F600}-3";
check(codePointString3.length === 4, 'code point length 3');
var quoteString3 = "\"\'3";
check(quoteString3.length === 3, 'quote escape length 3');
var slashString3 = "path\\3";
check(slashString3 === 'path\\3', 'slash escape string 3');
var nullEscapeString3 = "\0x3";
check(nullEscapeString3.length === 3, 'null escape length 3');
var continuedString3 = "left\
right3";
check(continuedString3 === 'leftright3', 'line continuation string 3');
check(sameString(plainString3) === 'literal-3', 'string argument 3');
var plainString4 = "literal-4";
check(plainString4 === 'literal-4', 'plain string 4');
check(plainString4.length === 9, 'plain length 4');
var singleString4 = 'single-4';
check(singleString4 === "single-4", 'single string 4');
var newlineString4 = "row\n4";
check(newlineString4.length === 5, 'newline length 4');
var hexEscapeString4 = "\x41-4";
check(hexEscapeString4 === 'A-4', 'hex escape string 4');
var unicodeEscapeString4 = "\u0041\u0042-4";
check(unicodeEscapeString4 === 'AB-4', 'unicode escape string 4');
var codePointString4 = "\u{1F600}-4";
check(codePointString4.length === 4, 'code point length 4');
var quoteString4 = "\"\'4";
check(quoteString4.length === 3, 'quote escape length 4');
var slashString4 = "path\\4";
check(slashString4 === 'path\\4', 'slash escape string 4');
var nullEscapeString4 = "\0x4";
check(nullEscapeString4.length === 3, 'null escape length 4');
var continuedString4 = "left\
right4";
check(continuedString4 === 'leftright4', 'line continuation string 4');
check(sameString(plainString4) === 'literal-4', 'string argument 4');
console.log('string-literals-evaluate-sv standard ' + score);
