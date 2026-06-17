// behavior: this-resolves-current-this-binding
// expected: pass
// goal: script
// size: standard
// variant: script.strict

var score = 0;
function assertThis(condition, name) {
if (!condition) { throw new Error(name); }
score = score + 1;
}
function strictIdentity() { "use strict"; return this; }
function sloppyIdentity() { return this; }
function receiverIdentity() { return this; }
function ArrowSource(value) { this.value = value; return (() => this); }
function Box(value) { this.value = value; this.self = this; }
function makeAccessorSlot(value) {
var target = { value: value, seen: null };
Object.defineProperty(target, "slot", {
get: function() { this.seen = this; return this.value; },
set: function(next) { this.seen = this; this.value = next; },
configurable: true
});
return target;
}
assertThis(sloppyIdentity() === undefined, "strict-plain-0");
assertThis(strictIdentity() === undefined, "strict-explicit-0");
var methodTarget0 = { value: 0, method: receiverIdentity };
assertThis(methodTarget0.method() === methodTarget0, "strict-method-0");
var detached0 = methodTarget0.method;
assertThis(detached0() === undefined, "strict-detached-0");
var callTarget0 = { value: 1 };
assertThis(receiverIdentity.call(callTarget0) === callTarget0, "strict-call-object-0");
assertThis(receiverIdentity.apply(callTarget0, []) === callTarget0, "strict-apply-object-0");
assertThis(receiverIdentity.call(null) === null, "strict-call-null-0");
assertThis(receiverIdentity.call(2) === 2, "strict-call-primitive-0");
var bound0 = receiverIdentity.bind(callTarget0);
assertThis(bound0() === callTarget0, "strict-bind-0");
var arrow0 = ArrowSource.call(callTarget0, 3);
assertThis(arrow0() === callTarget0, "strict-arrow-0");
var box0 = new Box(4);
assertThis(box0.self === box0 && box0.value === 4, "strict-constructor-0");
var access0 = makeAccessorSlot(5);
assertThis(access0.slot === 5 && access0.seen === access0, "strict-getter-0");
access0.slot = 6;
assertThis(access0.value === 6 && access0.seen === access0, "strict-setter-0");
assertThis(sloppyIdentity() === undefined, "strict-plain-1");
assertThis(strictIdentity() === undefined, "strict-explicit-1");
var methodTarget1 = { value: 1, method: receiverIdentity };
assertThis(methodTarget1.method() === methodTarget1, "strict-method-1");
var detached1 = methodTarget1.method;
assertThis(detached1() === undefined, "strict-detached-1");
var callTarget1 = { value: 2 };
assertThis(receiverIdentity.call(callTarget1) === callTarget1, "strict-call-object-1");
assertThis(receiverIdentity.apply(callTarget1, []) === callTarget1, "strict-apply-object-1");
assertThis(receiverIdentity.call(null) === null, "strict-call-null-1");
assertThis(receiverIdentity.call(3) === 3, "strict-call-primitive-1");
var bound1 = receiverIdentity.bind(callTarget1);
assertThis(bound1() === callTarget1, "strict-bind-1");
var arrow1 = ArrowSource.call(callTarget1, 4);
assertThis(arrow1() === callTarget1, "strict-arrow-1");
var box1 = new Box(5);
assertThis(box1.self === box1 && box1.value === 5, "strict-constructor-1");
var access1 = makeAccessorSlot(6);
assertThis(access1.slot === 6 && access1.seen === access1, "strict-getter-1");
access1.slot = 7;
assertThis(access1.value === 7 && access1.seen === access1, "strict-setter-1");
assertThis(sloppyIdentity() === undefined, "strict-plain-2");
assertThis(strictIdentity() === undefined, "strict-explicit-2");
var methodTarget2 = { value: 2, method: receiverIdentity };
assertThis(methodTarget2.method() === methodTarget2, "strict-method-2");
var detached2 = methodTarget2.method;
assertThis(detached2() === undefined, "strict-detached-2");
var callTarget2 = { value: 3 };
assertThis(receiverIdentity.call(callTarget2) === callTarget2, "strict-call-object-2");
assertThis(receiverIdentity.apply(callTarget2, []) === callTarget2, "strict-apply-object-2");
assertThis(receiverIdentity.call(null) === null, "strict-call-null-2");
assertThis(receiverIdentity.call(4) === 4, "strict-call-primitive-2");
var bound2 = receiverIdentity.bind(callTarget2);
assertThis(bound2() === callTarget2, "strict-bind-2");
var arrow2 = ArrowSource.call(callTarget2, 5);
assertThis(arrow2() === callTarget2, "strict-arrow-2");
var box2 = new Box(6);
assertThis(box2.self === box2 && box2.value === 6, "strict-constructor-2");
var access2 = makeAccessorSlot(7);
assertThis(access2.slot === 7 && access2.seen === access2, "strict-getter-2");
access2.slot = 8;
assertThis(access2.value === 8 && access2.seen === access2, "strict-setter-2");
assertThis(sloppyIdentity() === undefined, "strict-plain-3");
assertThis(strictIdentity() === undefined, "strict-explicit-3");
var methodTarget3 = { value: 3, method: receiverIdentity };
assertThis(methodTarget3.method() === methodTarget3, "strict-method-3");
var detached3 = methodTarget3.method;
assertThis(detached3() === undefined, "strict-detached-3");
var callTarget3 = { value: 4 };
assertThis(receiverIdentity.call(callTarget3) === callTarget3, "strict-call-object-3");
assertThis(receiverIdentity.apply(callTarget3, []) === callTarget3, "strict-apply-object-3");
assertThis(receiverIdentity.call(null) === null, "strict-call-null-3");
assertThis(receiverIdentity.call(5) === 5, "strict-call-primitive-3");
var bound3 = receiverIdentity.bind(callTarget3);
assertThis(bound3() === callTarget3, "strict-bind-3");
var arrow3 = ArrowSource.call(callTarget3, 6);
assertThis(arrow3() === callTarget3, "strict-arrow-3");
var box3 = new Box(7);
assertThis(box3.self === box3 && box3.value === 7, "strict-constructor-3");
var access3 = makeAccessorSlot(8);
assertThis(access3.slot === 8 && access3.seen === access3, "strict-getter-3");
access3.slot = 9;
assertThis(access3.value === 9 && access3.seen === access3, "strict-setter-3");
console.log("ok", score);
