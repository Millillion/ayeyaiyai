// behavior: this-resolves-current-this-binding
// expected: pass
// goal: module
// size: standard
// variant: module

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
assertThis(this === undefined, "module-top-level-this");
assertThis(sloppyIdentity() === undefined, "module-plain-0");
assertThis(strictIdentity() === undefined, "module-explicit-0");
var methodTarget0 = { value: 0, method: receiverIdentity };
assertThis(methodTarget0.method() === methodTarget0, "module-method-0");
var detached0 = methodTarget0.method;
assertThis(detached0() === undefined, "module-detached-0");
var callTarget0 = { value: 1 };
assertThis(receiverIdentity.call(callTarget0) === callTarget0, "module-call-object-0");
assertThis(receiverIdentity.apply(callTarget0, []) === callTarget0, "module-apply-object-0");
assertThis(receiverIdentity.call(undefined) === undefined, "module-call-undefined-0");
var bound0 = receiverIdentity.bind(callTarget0);
assertThis(bound0() === callTarget0, "module-bind-0");
var arrow0 = ArrowSource.call(callTarget0, 2);
assertThis(arrow0() === callTarget0, "module-arrow-0");
var box0 = new Box(3);
assertThis(box0.self === box0 && box0.value === 3, "module-constructor-0");
var access0 = makeAccessorSlot(4);
assertThis(access0.slot === 4 && access0.seen === access0, "module-getter-0");
access0.slot = 5;
assertThis(access0.value === 5 && access0.seen === access0, "module-setter-0");
assertThis(sloppyIdentity() === undefined, "module-plain-1");
assertThis(strictIdentity() === undefined, "module-explicit-1");
var methodTarget1 = { value: 1, method: receiverIdentity };
assertThis(methodTarget1.method() === methodTarget1, "module-method-1");
var detached1 = methodTarget1.method;
assertThis(detached1() === undefined, "module-detached-1");
var callTarget1 = { value: 2 };
assertThis(receiverIdentity.call(callTarget1) === callTarget1, "module-call-object-1");
assertThis(receiverIdentity.apply(callTarget1, []) === callTarget1, "module-apply-object-1");
assertThis(receiverIdentity.call(undefined) === undefined, "module-call-undefined-1");
var bound1 = receiverIdentity.bind(callTarget1);
assertThis(bound1() === callTarget1, "module-bind-1");
var arrow1 = ArrowSource.call(callTarget1, 3);
assertThis(arrow1() === callTarget1, "module-arrow-1");
var box1 = new Box(4);
assertThis(box1.self === box1 && box1.value === 4, "module-constructor-1");
var access1 = makeAccessorSlot(5);
assertThis(access1.slot === 5 && access1.seen === access1, "module-getter-1");
access1.slot = 6;
assertThis(access1.value === 6 && access1.seen === access1, "module-setter-1");
assertThis(sloppyIdentity() === undefined, "module-plain-2");
assertThis(strictIdentity() === undefined, "module-explicit-2");
var methodTarget2 = { value: 2, method: receiverIdentity };
assertThis(methodTarget2.method() === methodTarget2, "module-method-2");
var detached2 = methodTarget2.method;
assertThis(detached2() === undefined, "module-detached-2");
var callTarget2 = { value: 3 };
assertThis(receiverIdentity.call(callTarget2) === callTarget2, "module-call-object-2");
assertThis(receiverIdentity.apply(callTarget2, []) === callTarget2, "module-apply-object-2");
assertThis(receiverIdentity.call(undefined) === undefined, "module-call-undefined-2");
var bound2 = receiverIdentity.bind(callTarget2);
assertThis(bound2() === callTarget2, "module-bind-2");
var arrow2 = ArrowSource.call(callTarget2, 4);
assertThis(arrow2() === callTarget2, "module-arrow-2");
var box2 = new Box(5);
assertThis(box2.self === box2 && box2.value === 5, "module-constructor-2");
var access2 = makeAccessorSlot(6);
assertThis(access2.slot === 6 && access2.seen === access2, "module-getter-2");
access2.slot = 7;
assertThis(access2.value === 7 && access2.seen === access2, "module-setter-2");
assertThis(sloppyIdentity() === undefined, "module-plain-3");
assertThis(strictIdentity() === undefined, "module-explicit-3");
var methodTarget3 = { value: 3, method: receiverIdentity };
assertThis(methodTarget3.method() === methodTarget3, "module-method-3");
var detached3 = methodTarget3.method;
assertThis(detached3() === undefined, "module-detached-3");
var callTarget3 = { value: 4 };
assertThis(receiverIdentity.call(callTarget3) === callTarget3, "module-call-object-3");
assertThis(receiverIdentity.apply(callTarget3, []) === callTarget3, "module-apply-object-3");
assertThis(receiverIdentity.call(undefined) === undefined, "module-call-undefined-3");
var bound3 = receiverIdentity.bind(callTarget3);
assertThis(bound3() === callTarget3, "module-bind-3");
var arrow3 = ArrowSource.call(callTarget3, 5);
assertThis(arrow3() === callTarget3, "module-arrow-3");
var box3 = new Box(6);
assertThis(box3.self === box3 && box3.value === 6, "module-constructor-3");
var access3 = makeAccessorSlot(7);
assertThis(access3.slot === 7 && access3.seen === access3, "module-getter-3");
access3.slot = 8;
assertThis(access3.value === 8 && access3.seen === access3, "module-setter-3");
console.log("ok", score);
