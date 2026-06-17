// behavior: identifier-reference-resolves-nearest-binding
// expected: pass
// goal: script
// size: standard
// variant: script.sloppy

var shadow = 1;
var marker = 1000;
function __ayyRun() {
var score = 0;
var shadow = 10;
if (shadow !== 10) { throw "function shadow"; }
{ let shadow = 20; shadow = shadow + 1; if (shadow !== 21) { throw "block shadow"; } score = score + shadow; }
if (shadow !== 10) { throw "outer shadow after block"; }
function nested(shadow) { shadow = shadow + 3; if (shadow !== 33) { throw "parameter shadow"; } return shadow; }
score = score + nested(30);
if (shadow !== 10) { throw "outer shadow after nested"; }
score = score + shadow + marker;
{ let shadow = 40; shadow = shadow + 1; if (shadow !== 41) { throw "nearest 0"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker0 = marker + 0; if (marker0 !== 1000) { throw "marker 0"; } score = score + marker0; }
function nested0(shadow) { shadow = shadow + 1; return shadow; }
if (nested0(40) !== 41) { throw "nested 0"; }
{ let shadow = 41; shadow = shadow + 1; if (shadow !== 42) { throw "nearest 1"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker1 = marker + 1; if (marker1 !== 1001) { throw "marker 1"; } score = score + marker1; }
{ let shadow = 42; shadow = shadow + 1; if (shadow !== 43) { throw "nearest 2"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker2 = marker + 2; if (marker2 !== 1002) { throw "marker 2"; } score = score + marker2; }
{ let shadow = 43; shadow = shadow + 1; if (shadow !== 44) { throw "nearest 3"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker3 = marker + 3; if (marker3 !== 1003) { throw "marker 3"; } score = score + marker3; }
{ let shadow = 44; shadow = shadow + 1; if (shadow !== 45) { throw "nearest 4"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker4 = marker + 4; if (marker4 !== 1004) { throw "marker 4"; } score = score + marker4; }
{ let shadow = 45; shadow = shadow + 1; if (shadow !== 46) { throw "nearest 5"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker5 = marker + 5; if (marker5 !== 1005) { throw "marker 5"; } score = score + marker5; }
{ let shadow = 46; shadow = shadow + 1; if (shadow !== 47) { throw "nearest 6"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker6 = marker + 6; if (marker6 !== 1006) { throw "marker 6"; } score = score + marker6; }
{ let shadow = 47; shadow = shadow + 1; if (shadow !== 48) { throw "nearest 7"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker7 = marker + 7; if (marker7 !== 1007) { throw "marker 7"; } score = score + marker7; }
{ let shadow = 48; shadow = shadow + 1; if (shadow !== 49) { throw "nearest 8"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker8 = marker + 8; if (marker8 !== 1008) { throw "marker 8"; } score = score + marker8; }
{ let shadow = 49; shadow = shadow + 1; if (shadow !== 50) { throw "nearest 9"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker9 = marker + 9; if (marker9 !== 1009) { throw "marker 9"; } score = score + marker9; }
{ let shadow = 50; shadow = shadow + 1; if (shadow !== 51) { throw "nearest 10"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker10 = marker + 10; if (marker10 !== 1010) { throw "marker 10"; } score = score + marker10; }
{ let shadow = 51; shadow = shadow + 1; if (shadow !== 52) { throw "nearest 11"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker11 = marker + 11; if (marker11 !== 1011) { throw "marker 11"; } score = score + marker11; }
{ let shadow = 52; shadow = shadow + 1; if (shadow !== 53) { throw "nearest 12"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker12 = marker + 12; if (marker12 !== 1012) { throw "marker 12"; } score = score + marker12; }
{ let shadow = 53; shadow = shadow + 1; if (shadow !== 54) { throw "nearest 13"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker13 = marker + 13; if (marker13 !== 1013) { throw "marker 13"; } score = score + marker13; }
{ let shadow = 54; shadow = shadow + 1; if (shadow !== 55) { throw "nearest 14"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker14 = marker + 14; if (marker14 !== 1014) { throw "marker 14"; } score = score + marker14; }
{ let shadow = 55; shadow = shadow + 1; if (shadow !== 56) { throw "nearest 15"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker15 = marker + 15; if (marker15 !== 1015) { throw "marker 15"; } score = score + marker15; }
{ let shadow = 56; shadow = shadow + 1; if (shadow !== 57) { throw "nearest 16"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker16 = marker + 16; if (marker16 !== 1016) { throw "marker 16"; } score = score + marker16; }
{ let shadow = 57; shadow = shadow + 1; if (shadow !== 58) { throw "nearest 17"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker17 = marker + 17; if (marker17 !== 1017) { throw "marker 17"; } score = score + marker17; }
{ let shadow = 58; shadow = shadow + 1; if (shadow !== 59) { throw "nearest 18"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker18 = marker + 18; if (marker18 !== 1018) { throw "marker 18"; } score = score + marker18; }
{ let shadow = 59; shadow = shadow + 1; if (shadow !== 60) { throw "nearest 19"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker19 = marker + 19; if (marker19 !== 1019) { throw "marker 19"; } score = score + marker19; }
function nested19(shadow) { shadow = shadow + 20; return shadow; }
if (nested19(59) !== 79) { throw "nested 19"; }
{ let shadow = 60; shadow = shadow + 1; if (shadow !== 61) { throw "nearest 20"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker20 = marker + 20; if (marker20 !== 1020) { throw "marker 20"; } score = score + marker20; }
{ let shadow = 61; shadow = shadow + 1; if (shadow !== 62) { throw "nearest 21"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker21 = marker + 21; if (marker21 !== 1021) { throw "marker 21"; } score = score + marker21; }
{ let shadow = 62; shadow = shadow + 1; if (shadow !== 63) { throw "nearest 22"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker22 = marker + 22; if (marker22 !== 1022) { throw "marker 22"; } score = score + marker22; }
{ let shadow = 63; shadow = shadow + 1; if (shadow !== 64) { throw "nearest 23"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker23 = marker + 23; if (marker23 !== 1023) { throw "marker 23"; } score = score + marker23; }
{ let shadow = 64; shadow = shadow + 1; if (shadow !== 65) { throw "nearest 24"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker24 = marker + 24; if (marker24 !== 1024) { throw "marker 24"; } score = score + marker24; }
{ let shadow = 65; shadow = shadow + 1; if (shadow !== 66) { throw "nearest 25"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker25 = marker + 25; if (marker25 !== 1025) { throw "marker 25"; } score = score + marker25; }
{ let shadow = 66; shadow = shadow + 1; if (shadow !== 67) { throw "nearest 26"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker26 = marker + 26; if (marker26 !== 1026) { throw "marker 26"; } score = score + marker26; }
return score;
}
console.log("ok", __ayyRun(), shadow, marker);
