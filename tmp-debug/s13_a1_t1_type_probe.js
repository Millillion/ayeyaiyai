var __MONSTER = "monster";

function __PROTO() {}
__PROTO.type = __MONSTER;

function __FACTORY() {}
__FACTORY.prototype = __PROTO;

var __monster = new __FACTORY();
console.log(__monster.type);
