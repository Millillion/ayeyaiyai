// behavior: identifier-reference-resolves-nearest-binding
// expected: pass
// goal: script
// size: stress
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
{ let shadow = 67; shadow = shadow + 1; if (shadow !== 68) { throw "nearest 27"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker27 = marker + 27; if (marker27 !== 1027) { throw "marker 27"; } score = score + marker27; }
{ let shadow = 68; shadow = shadow + 1; if (shadow !== 69) { throw "nearest 28"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker28 = marker + 28; if (marker28 !== 1028) { throw "marker 28"; } score = score + marker28; }
{ let shadow = 69; shadow = shadow + 1; if (shadow !== 70) { throw "nearest 29"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker29 = marker + 29; if (marker29 !== 1029) { throw "marker 29"; } score = score + marker29; }
{ let shadow = 70; shadow = shadow + 1; if (shadow !== 71) { throw "nearest 30"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker30 = marker + 30; if (marker30 !== 1030) { throw "marker 30"; } score = score + marker30; }
{ let shadow = 71; shadow = shadow + 1; if (shadow !== 72) { throw "nearest 31"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker31 = marker + 31; if (marker31 !== 1031) { throw "marker 31"; } score = score + marker31; }
{ let shadow = 72; shadow = shadow + 1; if (shadow !== 73) { throw "nearest 32"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker32 = marker + 32; if (marker32 !== 1032) { throw "marker 32"; } score = score + marker32; }
{ let shadow = 73; shadow = shadow + 1; if (shadow !== 74) { throw "nearest 33"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker33 = marker + 33; if (marker33 !== 1033) { throw "marker 33"; } score = score + marker33; }
{ let shadow = 74; shadow = shadow + 1; if (shadow !== 75) { throw "nearest 34"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker34 = marker + 34; if (marker34 !== 1034) { throw "marker 34"; } score = score + marker34; }
{ let shadow = 75; shadow = shadow + 1; if (shadow !== 76) { throw "nearest 35"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker35 = marker + 35; if (marker35 !== 1035) { throw "marker 35"; } score = score + marker35; }
{ let shadow = 76; shadow = shadow + 1; if (shadow !== 77) { throw "nearest 36"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker36 = marker + 36; if (marker36 !== 1036) { throw "marker 36"; } score = score + marker36; }
{ let shadow = 77; shadow = shadow + 1; if (shadow !== 78) { throw "nearest 37"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker37 = marker + 37; if (marker37 !== 1037) { throw "marker 37"; } score = score + marker37; }
{ let shadow = 78; shadow = shadow + 1; if (shadow !== 79) { throw "nearest 38"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker38 = marker + 38; if (marker38 !== 1038) { throw "marker 38"; } score = score + marker38; }
function nested38(shadow) { shadow = shadow + 39; return shadow; }
if (nested38(78) !== 117) { throw "nested 38"; }
{ let shadow = 79; shadow = shadow + 1; if (shadow !== 80) { throw "nearest 39"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker39 = marker + 39; if (marker39 !== 1039) { throw "marker 39"; } score = score + marker39; }
{ let shadow = 80; shadow = shadow + 1; if (shadow !== 81) { throw "nearest 40"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker40 = marker + 40; if (marker40 !== 1040) { throw "marker 40"; } score = score + marker40; }
{ let shadow = 81; shadow = shadow + 1; if (shadow !== 82) { throw "nearest 41"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker41 = marker + 41; if (marker41 !== 1041) { throw "marker 41"; } score = score + marker41; }
{ let shadow = 82; shadow = shadow + 1; if (shadow !== 83) { throw "nearest 42"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker42 = marker + 42; if (marker42 !== 1042) { throw "marker 42"; } score = score + marker42; }
{ let shadow = 83; shadow = shadow + 1; if (shadow !== 84) { throw "nearest 43"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker43 = marker + 43; if (marker43 !== 1043) { throw "marker 43"; } score = score + marker43; }
{ let shadow = 84; shadow = shadow + 1; if (shadow !== 85) { throw "nearest 44"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker44 = marker + 44; if (marker44 !== 1044) { throw "marker 44"; } score = score + marker44; }
{ let shadow = 85; shadow = shadow + 1; if (shadow !== 86) { throw "nearest 45"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker45 = marker + 45; if (marker45 !== 1045) { throw "marker 45"; } score = score + marker45; }
{ let shadow = 86; shadow = shadow + 1; if (shadow !== 87) { throw "nearest 46"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker46 = marker + 46; if (marker46 !== 1046) { throw "marker 46"; } score = score + marker46; }
{ let shadow = 87; shadow = shadow + 1; if (shadow !== 88) { throw "nearest 47"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker47 = marker + 47; if (marker47 !== 1047) { throw "marker 47"; } score = score + marker47; }
{ let shadow = 88; shadow = shadow + 1; if (shadow !== 89) { throw "nearest 48"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker48 = marker + 48; if (marker48 !== 1048) { throw "marker 48"; } score = score + marker48; }
{ let shadow = 89; shadow = shadow + 1; if (shadow !== 90) { throw "nearest 49"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker49 = marker + 49; if (marker49 !== 1049) { throw "marker 49"; } score = score + marker49; }
{ let shadow = 90; shadow = shadow + 1; if (shadow !== 91) { throw "nearest 50"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker50 = marker + 50; if (marker50 !== 1050) { throw "marker 50"; } score = score + marker50; }
{ let shadow = 91; shadow = shadow + 1; if (shadow !== 92) { throw "nearest 51"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker51 = marker + 51; if (marker51 !== 1051) { throw "marker 51"; } score = score + marker51; }
{ let shadow = 92; shadow = shadow + 1; if (shadow !== 93) { throw "nearest 52"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker52 = marker + 52; if (marker52 !== 1052) { throw "marker 52"; } score = score + marker52; }
{ let shadow = 93; shadow = shadow + 1; if (shadow !== 94) { throw "nearest 53"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker53 = marker + 53; if (marker53 !== 1053) { throw "marker 53"; } score = score + marker53; }
{ let shadow = 94; shadow = shadow + 1; if (shadow !== 95) { throw "nearest 54"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker54 = marker + 54; if (marker54 !== 1054) { throw "marker 54"; } score = score + marker54; }
{ let shadow = 95; shadow = shadow + 1; if (shadow !== 96) { throw "nearest 55"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker55 = marker + 55; if (marker55 !== 1055) { throw "marker 55"; } score = score + marker55; }
{ let shadow = 96; shadow = shadow + 1; if (shadow !== 97) { throw "nearest 56"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker56 = marker + 56; if (marker56 !== 1056) { throw "marker 56"; } score = score + marker56; }
{ let shadow = 97; shadow = shadow + 1; if (shadow !== 98) { throw "nearest 57"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker57 = marker + 57; if (marker57 !== 1057) { throw "marker 57"; } score = score + marker57; }
function nested57(shadow) { shadow = shadow + 58; return shadow; }
if (nested57(97) !== 155) { throw "nested 57"; }
{ let shadow = 98; shadow = shadow + 1; if (shadow !== 99) { throw "nearest 58"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker58 = marker + 58; if (marker58 !== 1058) { throw "marker 58"; } score = score + marker58; }
{ let shadow = 99; shadow = shadow + 1; if (shadow !== 100) { throw "nearest 59"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker59 = marker + 59; if (marker59 !== 1059) { throw "marker 59"; } score = score + marker59; }
{ let shadow = 100; shadow = shadow + 1; if (shadow !== 101) { throw "nearest 60"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker60 = marker + 60; if (marker60 !== 1060) { throw "marker 60"; } score = score + marker60; }
{ let shadow = 101; shadow = shadow + 1; if (shadow !== 102) { throw "nearest 61"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker61 = marker + 61; if (marker61 !== 1061) { throw "marker 61"; } score = score + marker61; }
{ let shadow = 102; shadow = shadow + 1; if (shadow !== 103) { throw "nearest 62"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker62 = marker + 62; if (marker62 !== 1062) { throw "marker 62"; } score = score + marker62; }
{ let shadow = 103; shadow = shadow + 1; if (shadow !== 104) { throw "nearest 63"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker63 = marker + 63; if (marker63 !== 1063) { throw "marker 63"; } score = score + marker63; }
{ let shadow = 104; shadow = shadow + 1; if (shadow !== 105) { throw "nearest 64"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker64 = marker + 64; if (marker64 !== 1064) { throw "marker 64"; } score = score + marker64; }
{ let shadow = 105; shadow = shadow + 1; if (shadow !== 106) { throw "nearest 65"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker65 = marker + 65; if (marker65 !== 1065) { throw "marker 65"; } score = score + marker65; }
{ let shadow = 106; shadow = shadow + 1; if (shadow !== 107) { throw "nearest 66"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker66 = marker + 66; if (marker66 !== 1066) { throw "marker 66"; } score = score + marker66; }
{ let shadow = 107; shadow = shadow + 1; if (shadow !== 108) { throw "nearest 67"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker67 = marker + 67; if (marker67 !== 1067) { throw "marker 67"; } score = score + marker67; }
{ let shadow = 108; shadow = shadow + 1; if (shadow !== 109) { throw "nearest 68"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker68 = marker + 68; if (marker68 !== 1068) { throw "marker 68"; } score = score + marker68; }
{ let shadow = 109; shadow = shadow + 1; if (shadow !== 110) { throw "nearest 69"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker69 = marker + 69; if (marker69 !== 1069) { throw "marker 69"; } score = score + marker69; }
{ let shadow = 110; shadow = shadow + 1; if (shadow !== 111) { throw "nearest 70"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker70 = marker + 70; if (marker70 !== 1070) { throw "marker 70"; } score = score + marker70; }
{ let shadow = 111; shadow = shadow + 1; if (shadow !== 112) { throw "nearest 71"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker71 = marker + 71; if (marker71 !== 1071) { throw "marker 71"; } score = score + marker71; }
{ let shadow = 112; shadow = shadow + 1; if (shadow !== 113) { throw "nearest 72"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker72 = marker + 72; if (marker72 !== 1072) { throw "marker 72"; } score = score + marker72; }
{ let shadow = 113; shadow = shadow + 1; if (shadow !== 114) { throw "nearest 73"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker73 = marker + 73; if (marker73 !== 1073) { throw "marker 73"; } score = score + marker73; }
{ let shadow = 114; shadow = shadow + 1; if (shadow !== 115) { throw "nearest 74"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker74 = marker + 74; if (marker74 !== 1074) { throw "marker 74"; } score = score + marker74; }
{ let shadow = 115; shadow = shadow + 1; if (shadow !== 116) { throw "nearest 75"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker75 = marker + 75; if (marker75 !== 1075) { throw "marker 75"; } score = score + marker75; }
{ let shadow = 116; shadow = shadow + 1; if (shadow !== 117) { throw "nearest 76"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker76 = marker + 76; if (marker76 !== 1076) { throw "marker 76"; } score = score + marker76; }
function nested76(shadow) { shadow = shadow + 77; return shadow; }
if (nested76(116) !== 193) { throw "nested 76"; }
{ let shadow = 117; shadow = shadow + 1; if (shadow !== 118) { throw "nearest 77"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker77 = marker + 77; if (marker77 !== 1077) { throw "marker 77"; } score = score + marker77; }
{ let shadow = 118; shadow = shadow + 1; if (shadow !== 119) { throw "nearest 78"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker78 = marker + 78; if (marker78 !== 1078) { throw "marker 78"; } score = score + marker78; }
{ let shadow = 119; shadow = shadow + 1; if (shadow !== 120) { throw "nearest 79"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker79 = marker + 79; if (marker79 !== 1079) { throw "marker 79"; } score = score + marker79; }
{ let shadow = 120; shadow = shadow + 1; if (shadow !== 121) { throw "nearest 80"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker80 = marker + 80; if (marker80 !== 1080) { throw "marker 80"; } score = score + marker80; }
{ let shadow = 121; shadow = shadow + 1; if (shadow !== 122) { throw "nearest 81"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker81 = marker + 81; if (marker81 !== 1081) { throw "marker 81"; } score = score + marker81; }
{ let shadow = 122; shadow = shadow + 1; if (shadow !== 123) { throw "nearest 82"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker82 = marker + 82; if (marker82 !== 1082) { throw "marker 82"; } score = score + marker82; }
{ let shadow = 123; shadow = shadow + 1; if (shadow !== 124) { throw "nearest 83"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker83 = marker + 83; if (marker83 !== 1083) { throw "marker 83"; } score = score + marker83; }
{ let shadow = 124; shadow = shadow + 1; if (shadow !== 125) { throw "nearest 84"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker84 = marker + 84; if (marker84 !== 1084) { throw "marker 84"; } score = score + marker84; }
{ let shadow = 125; shadow = shadow + 1; if (shadow !== 126) { throw "nearest 85"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker85 = marker + 85; if (marker85 !== 1085) { throw "marker 85"; } score = score + marker85; }
{ let shadow = 126; shadow = shadow + 1; if (shadow !== 127) { throw "nearest 86"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker86 = marker + 86; if (marker86 !== 1086) { throw "marker 86"; } score = score + marker86; }
{ let shadow = 127; shadow = shadow + 1; if (shadow !== 128) { throw "nearest 87"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker87 = marker + 87; if (marker87 !== 1087) { throw "marker 87"; } score = score + marker87; }
{ let shadow = 128; shadow = shadow + 1; if (shadow !== 129) { throw "nearest 88"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker88 = marker + 88; if (marker88 !== 1088) { throw "marker 88"; } score = score + marker88; }
{ let shadow = 129; shadow = shadow + 1; if (shadow !== 130) { throw "nearest 89"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker89 = marker + 89; if (marker89 !== 1089) { throw "marker 89"; } score = score + marker89; }
{ let shadow = 130; shadow = shadow + 1; if (shadow !== 131) { throw "nearest 90"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker90 = marker + 90; if (marker90 !== 1090) { throw "marker 90"; } score = score + marker90; }
{ let shadow = 131; shadow = shadow + 1; if (shadow !== 132) { throw "nearest 91"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker91 = marker + 91; if (marker91 !== 1091) { throw "marker 91"; } score = score + marker91; }
{ let shadow = 132; shadow = shadow + 1; if (shadow !== 133) { throw "nearest 92"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker92 = marker + 92; if (marker92 !== 1092) { throw "marker 92"; } score = score + marker92; }
{ let shadow = 133; shadow = shadow + 1; if (shadow !== 134) { throw "nearest 93"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker93 = marker + 93; if (marker93 !== 1093) { throw "marker 93"; } score = score + marker93; }
{ let shadow = 134; shadow = shadow + 1; if (shadow !== 135) { throw "nearest 94"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker94 = marker + 94; if (marker94 !== 1094) { throw "marker 94"; } score = score + marker94; }
{ let shadow = 135; shadow = shadow + 1; if (shadow !== 136) { throw "nearest 95"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker95 = marker + 95; if (marker95 !== 1095) { throw "marker 95"; } score = score + marker95; }
function nested95(shadow) { shadow = shadow + 96; return shadow; }
if (nested95(135) !== 231) { throw "nested 95"; }
{ let shadow = 136; shadow = shadow + 1; if (shadow !== 137) { throw "nearest 96"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker96 = marker + 96; if (marker96 !== 1096) { throw "marker 96"; } score = score + marker96; }
{ let shadow = 137; shadow = shadow + 1; if (shadow !== 138) { throw "nearest 97"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker97 = marker + 97; if (marker97 !== 1097) { throw "marker 97"; } score = score + marker97; }
{ let shadow = 138; shadow = shadow + 1; if (shadow !== 139) { throw "nearest 98"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker98 = marker + 98; if (marker98 !== 1098) { throw "marker 98"; } score = score + marker98; }
{ let shadow = 139; shadow = shadow + 1; if (shadow !== 140) { throw "nearest 99"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker99 = marker + 99; if (marker99 !== 1099) { throw "marker 99"; } score = score + marker99; }
{ let shadow = 140; shadow = shadow + 1; if (shadow !== 141) { throw "nearest 100"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker100 = marker + 100; if (marker100 !== 1100) { throw "marker 100"; } score = score + marker100; }
{ let shadow = 141; shadow = shadow + 1; if (shadow !== 142) { throw "nearest 101"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker101 = marker + 101; if (marker101 !== 1101) { throw "marker 101"; } score = score + marker101; }
{ let shadow = 142; shadow = shadow + 1; if (shadow !== 143) { throw "nearest 102"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker102 = marker + 102; if (marker102 !== 1102) { throw "marker 102"; } score = score + marker102; }
{ let shadow = 143; shadow = shadow + 1; if (shadow !== 144) { throw "nearest 103"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker103 = marker + 103; if (marker103 !== 1103) { throw "marker 103"; } score = score + marker103; }
{ let shadow = 144; shadow = shadow + 1; if (shadow !== 145) { throw "nearest 104"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker104 = marker + 104; if (marker104 !== 1104) { throw "marker 104"; } score = score + marker104; }
{ let shadow = 145; shadow = shadow + 1; if (shadow !== 146) { throw "nearest 105"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker105 = marker + 105; if (marker105 !== 1105) { throw "marker 105"; } score = score + marker105; }
{ let shadow = 146; shadow = shadow + 1; if (shadow !== 147) { throw "nearest 106"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker106 = marker + 106; if (marker106 !== 1106) { throw "marker 106"; } score = score + marker106; }
{ let shadow = 147; shadow = shadow + 1; if (shadow !== 148) { throw "nearest 107"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker107 = marker + 107; if (marker107 !== 1107) { throw "marker 107"; } score = score + marker107; }
{ let shadow = 148; shadow = shadow + 1; if (shadow !== 149) { throw "nearest 108"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker108 = marker + 108; if (marker108 !== 1108) { throw "marker 108"; } score = score + marker108; }
{ let shadow = 149; shadow = shadow + 1; if (shadow !== 150) { throw "nearest 109"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker109 = marker + 109; if (marker109 !== 1109) { throw "marker 109"; } score = score + marker109; }
{ let shadow = 150; shadow = shadow + 1; if (shadow !== 151) { throw "nearest 110"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker110 = marker + 110; if (marker110 !== 1110) { throw "marker 110"; } score = score + marker110; }
{ let shadow = 151; shadow = shadow + 1; if (shadow !== 152) { throw "nearest 111"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker111 = marker + 111; if (marker111 !== 1111) { throw "marker 111"; } score = score + marker111; }
{ let shadow = 152; shadow = shadow + 1; if (shadow !== 153) { throw "nearest 112"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker112 = marker + 112; if (marker112 !== 1112) { throw "marker 112"; } score = score + marker112; }
{ let shadow = 153; shadow = shadow + 1; if (shadow !== 154) { throw "nearest 113"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker113 = marker + 113; if (marker113 !== 1113) { throw "marker 113"; } score = score + marker113; }
{ let shadow = 154; shadow = shadow + 1; if (shadow !== 155) { throw "nearest 114"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker114 = marker + 114; if (marker114 !== 1114) { throw "marker 114"; } score = score + marker114; }
function nested114(shadow) { shadow = shadow + 115; return shadow; }
if (nested114(154) !== 269) { throw "nested 114"; }
{ let shadow = 155; shadow = shadow + 1; if (shadow !== 156) { throw "nearest 115"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker115 = marker + 115; if (marker115 !== 1115) { throw "marker 115"; } score = score + marker115; }
{ let shadow = 156; shadow = shadow + 1; if (shadow !== 157) { throw "nearest 116"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker116 = marker + 116; if (marker116 !== 1116) { throw "marker 116"; } score = score + marker116; }
{ let shadow = 157; shadow = shadow + 1; if (shadow !== 158) { throw "nearest 117"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker117 = marker + 117; if (marker117 !== 1117) { throw "marker 117"; } score = score + marker117; }
{ let shadow = 158; shadow = shadow + 1; if (shadow !== 159) { throw "nearest 118"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker118 = marker + 118; if (marker118 !== 1118) { throw "marker 118"; } score = score + marker118; }
{ let shadow = 159; shadow = shadow + 1; if (shadow !== 160) { throw "nearest 119"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker119 = marker + 119; if (marker119 !== 1119) { throw "marker 119"; } score = score + marker119; }
{ let shadow = 160; shadow = shadow + 1; if (shadow !== 161) { throw "nearest 120"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker120 = marker + 120; if (marker120 !== 1120) { throw "marker 120"; } score = score + marker120; }
{ let shadow = 161; shadow = shadow + 1; if (shadow !== 162) { throw "nearest 121"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker121 = marker + 121; if (marker121 !== 1121) { throw "marker 121"; } score = score + marker121; }
{ let shadow = 162; shadow = shadow + 1; if (shadow !== 163) { throw "nearest 122"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker122 = marker + 122; if (marker122 !== 1122) { throw "marker 122"; } score = score + marker122; }
{ let shadow = 163; shadow = shadow + 1; if (shadow !== 164) { throw "nearest 123"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker123 = marker + 123; if (marker123 !== 1123) { throw "marker 123"; } score = score + marker123; }
{ let shadow = 164; shadow = shadow + 1; if (shadow !== 165) { throw "nearest 124"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker124 = marker + 124; if (marker124 !== 1124) { throw "marker 124"; } score = score + marker124; }
{ let shadow = 165; shadow = shadow + 1; if (shadow !== 166) { throw "nearest 125"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker125 = marker + 125; if (marker125 !== 1125) { throw "marker 125"; } score = score + marker125; }
{ let shadow = 166; shadow = shadow + 1; if (shadow !== 167) { throw "nearest 126"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker126 = marker + 126; if (marker126 !== 1126) { throw "marker 126"; } score = score + marker126; }
{ let shadow = 167; shadow = shadow + 1; if (shadow !== 168) { throw "nearest 127"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker127 = marker + 127; if (marker127 !== 1127) { throw "marker 127"; } score = score + marker127; }
{ let shadow = 168; shadow = shadow + 1; if (shadow !== 169) { throw "nearest 128"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker128 = marker + 128; if (marker128 !== 1128) { throw "marker 128"; } score = score + marker128; }
{ let shadow = 169; shadow = shadow + 1; if (shadow !== 170) { throw "nearest 129"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker129 = marker + 129; if (marker129 !== 1129) { throw "marker 129"; } score = score + marker129; }
{ let shadow = 170; shadow = shadow + 1; if (shadow !== 171) { throw "nearest 130"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker130 = marker + 130; if (marker130 !== 1130) { throw "marker 130"; } score = score + marker130; }
{ let shadow = 171; shadow = shadow + 1; if (shadow !== 172) { throw "nearest 131"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker131 = marker + 131; if (marker131 !== 1131) { throw "marker 131"; } score = score + marker131; }
{ let shadow = 172; shadow = shadow + 1; if (shadow !== 173) { throw "nearest 132"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker132 = marker + 132; if (marker132 !== 1132) { throw "marker 132"; } score = score + marker132; }
{ let shadow = 173; shadow = shadow + 1; if (shadow !== 174) { throw "nearest 133"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker133 = marker + 133; if (marker133 !== 1133) { throw "marker 133"; } score = score + marker133; }
function nested133(shadow) { shadow = shadow + 134; return shadow; }
if (nested133(173) !== 307) { throw "nested 133"; }
{ let shadow = 174; shadow = shadow + 1; if (shadow !== 175) { throw "nearest 134"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker134 = marker + 134; if (marker134 !== 1134) { throw "marker 134"; } score = score + marker134; }
{ let shadow = 175; shadow = shadow + 1; if (shadow !== 176) { throw "nearest 135"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker135 = marker + 135; if (marker135 !== 1135) { throw "marker 135"; } score = score + marker135; }
{ let shadow = 176; shadow = shadow + 1; if (shadow !== 177) { throw "nearest 136"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker136 = marker + 136; if (marker136 !== 1136) { throw "marker 136"; } score = score + marker136; }
{ let shadow = 177; shadow = shadow + 1; if (shadow !== 178) { throw "nearest 137"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker137 = marker + 137; if (marker137 !== 1137) { throw "marker 137"; } score = score + marker137; }
{ let shadow = 178; shadow = shadow + 1; if (shadow !== 179) { throw "nearest 138"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker138 = marker + 138; if (marker138 !== 1138) { throw "marker 138"; } score = score + marker138; }
{ let shadow = 179; shadow = shadow + 1; if (shadow !== 180) { throw "nearest 139"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker139 = marker + 139; if (marker139 !== 1139) { throw "marker 139"; } score = score + marker139; }
{ let shadow = 180; shadow = shadow + 1; if (shadow !== 181) { throw "nearest 140"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker140 = marker + 140; if (marker140 !== 1140) { throw "marker 140"; } score = score + marker140; }
{ let shadow = 181; shadow = shadow + 1; if (shadow !== 182) { throw "nearest 141"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker141 = marker + 141; if (marker141 !== 1141) { throw "marker 141"; } score = score + marker141; }
{ let shadow = 182; shadow = shadow + 1; if (shadow !== 183) { throw "nearest 142"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker142 = marker + 142; if (marker142 !== 1142) { throw "marker 142"; } score = score + marker142; }
{ let shadow = 183; shadow = shadow + 1; if (shadow !== 184) { throw "nearest 143"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker143 = marker + 143; if (marker143 !== 1143) { throw "marker 143"; } score = score + marker143; }
{ let shadow = 184; shadow = shadow + 1; if (shadow !== 185) { throw "nearest 144"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker144 = marker + 144; if (marker144 !== 1144) { throw "marker 144"; } score = score + marker144; }
{ let shadow = 185; shadow = shadow + 1; if (shadow !== 186) { throw "nearest 145"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker145 = marker + 145; if (marker145 !== 1145) { throw "marker 145"; } score = score + marker145; }
{ let shadow = 186; shadow = shadow + 1; if (shadow !== 187) { throw "nearest 146"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker146 = marker + 146; if (marker146 !== 1146) { throw "marker 146"; } score = score + marker146; }
{ let shadow = 187; shadow = shadow + 1; if (shadow !== 188) { throw "nearest 147"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker147 = marker + 147; if (marker147 !== 1147) { throw "marker 147"; } score = score + marker147; }
{ let shadow = 188; shadow = shadow + 1; if (shadow !== 189) { throw "nearest 148"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker148 = marker + 148; if (marker148 !== 1148) { throw "marker 148"; } score = score + marker148; }
{ let shadow = 189; shadow = shadow + 1; if (shadow !== 190) { throw "nearest 149"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker149 = marker + 149; if (marker149 !== 1149) { throw "marker 149"; } score = score + marker149; }
{ let shadow = 190; shadow = shadow + 1; if (shadow !== 191) { throw "nearest 150"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker150 = marker + 150; if (marker150 !== 1150) { throw "marker 150"; } score = score + marker150; }
{ let shadow = 191; shadow = shadow + 1; if (shadow !== 192) { throw "nearest 151"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker151 = marker + 151; if (marker151 !== 1151) { throw "marker 151"; } score = score + marker151; }
{ let shadow = 192; shadow = shadow + 1; if (shadow !== 193) { throw "nearest 152"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker152 = marker + 152; if (marker152 !== 1152) { throw "marker 152"; } score = score + marker152; }
function nested152(shadow) { shadow = shadow + 153; return shadow; }
if (nested152(192) !== 345) { throw "nested 152"; }
{ let shadow = 193; shadow = shadow + 1; if (shadow !== 194) { throw "nearest 153"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker153 = marker + 153; if (marker153 !== 1153) { throw "marker 153"; } score = score + marker153; }
{ let shadow = 194; shadow = shadow + 1; if (shadow !== 195) { throw "nearest 154"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker154 = marker + 154; if (marker154 !== 1154) { throw "marker 154"; } score = score + marker154; }
{ let shadow = 195; shadow = shadow + 1; if (shadow !== 196) { throw "nearest 155"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker155 = marker + 155; if (marker155 !== 1155) { throw "marker 155"; } score = score + marker155; }
{ let shadow = 196; shadow = shadow + 1; if (shadow !== 197) { throw "nearest 156"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker156 = marker + 156; if (marker156 !== 1156) { throw "marker 156"; } score = score + marker156; }
{ let shadow = 197; shadow = shadow + 1; if (shadow !== 198) { throw "nearest 157"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker157 = marker + 157; if (marker157 !== 1157) { throw "marker 157"; } score = score + marker157; }
{ let shadow = 198; shadow = shadow + 1; if (shadow !== 199) { throw "nearest 158"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker158 = marker + 158; if (marker158 !== 1158) { throw "marker 158"; } score = score + marker158; }
{ let shadow = 199; shadow = shadow + 1; if (shadow !== 200) { throw "nearest 159"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker159 = marker + 159; if (marker159 !== 1159) { throw "marker 159"; } score = score + marker159; }
{ let shadow = 200; shadow = shadow + 1; if (shadow !== 201) { throw "nearest 160"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker160 = marker + 160; if (marker160 !== 1160) { throw "marker 160"; } score = score + marker160; }
{ let shadow = 201; shadow = shadow + 1; if (shadow !== 202) { throw "nearest 161"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker161 = marker + 161; if (marker161 !== 1161) { throw "marker 161"; } score = score + marker161; }
{ let shadow = 202; shadow = shadow + 1; if (shadow !== 203) { throw "nearest 162"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker162 = marker + 162; if (marker162 !== 1162) { throw "marker 162"; } score = score + marker162; }
{ let shadow = 203; shadow = shadow + 1; if (shadow !== 204) { throw "nearest 163"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker163 = marker + 163; if (marker163 !== 1163) { throw "marker 163"; } score = score + marker163; }
{ let shadow = 204; shadow = shadow + 1; if (shadow !== 205) { throw "nearest 164"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker164 = marker + 164; if (marker164 !== 1164) { throw "marker 164"; } score = score + marker164; }
{ let shadow = 205; shadow = shadow + 1; if (shadow !== 206) { throw "nearest 165"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker165 = marker + 165; if (marker165 !== 1165) { throw "marker 165"; } score = score + marker165; }
{ let shadow = 206; shadow = shadow + 1; if (shadow !== 207) { throw "nearest 166"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker166 = marker + 166; if (marker166 !== 1166) { throw "marker 166"; } score = score + marker166; }
{ let shadow = 207; shadow = shadow + 1; if (shadow !== 208) { throw "nearest 167"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker167 = marker + 167; if (marker167 !== 1167) { throw "marker 167"; } score = score + marker167; }
{ let shadow = 208; shadow = shadow + 1; if (shadow !== 209) { throw "nearest 168"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker168 = marker + 168; if (marker168 !== 1168) { throw "marker 168"; } score = score + marker168; }
{ let shadow = 209; shadow = shadow + 1; if (shadow !== 210) { throw "nearest 169"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker169 = marker + 169; if (marker169 !== 1169) { throw "marker 169"; } score = score + marker169; }
{ let shadow = 210; shadow = shadow + 1; if (shadow !== 211) { throw "nearest 170"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker170 = marker + 170; if (marker170 !== 1170) { throw "marker 170"; } score = score + marker170; }
{ let shadow = 211; shadow = shadow + 1; if (shadow !== 212) { throw "nearest 171"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker171 = marker + 171; if (marker171 !== 1171) { throw "marker 171"; } score = score + marker171; }
function nested171(shadow) { shadow = shadow + 172; return shadow; }
if (nested171(211) !== 383) { throw "nested 171"; }
{ let shadow = 212; shadow = shadow + 1; if (shadow !== 213) { throw "nearest 172"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker172 = marker + 172; if (marker172 !== 1172) { throw "marker 172"; } score = score + marker172; }
{ let shadow = 213; shadow = shadow + 1; if (shadow !== 214) { throw "nearest 173"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker173 = marker + 173; if (marker173 !== 1173) { throw "marker 173"; } score = score + marker173; }
{ let shadow = 214; shadow = shadow + 1; if (shadow !== 215) { throw "nearest 174"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker174 = marker + 174; if (marker174 !== 1174) { throw "marker 174"; } score = score + marker174; }
{ let shadow = 215; shadow = shadow + 1; if (shadow !== 216) { throw "nearest 175"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker175 = marker + 175; if (marker175 !== 1175) { throw "marker 175"; } score = score + marker175; }
{ let shadow = 216; shadow = shadow + 1; if (shadow !== 217) { throw "nearest 176"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker176 = marker + 176; if (marker176 !== 1176) { throw "marker 176"; } score = score + marker176; }
{ let shadow = 217; shadow = shadow + 1; if (shadow !== 218) { throw "nearest 177"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker177 = marker + 177; if (marker177 !== 1177) { throw "marker 177"; } score = score + marker177; }
{ let shadow = 218; shadow = shadow + 1; if (shadow !== 219) { throw "nearest 178"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker178 = marker + 178; if (marker178 !== 1178) { throw "marker 178"; } score = score + marker178; }
{ let shadow = 219; shadow = shadow + 1; if (shadow !== 220) { throw "nearest 179"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker179 = marker + 179; if (marker179 !== 1179) { throw "marker 179"; } score = score + marker179; }
{ let shadow = 220; shadow = shadow + 1; if (shadow !== 221) { throw "nearest 180"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker180 = marker + 180; if (marker180 !== 1180) { throw "marker 180"; } score = score + marker180; }
{ let shadow = 221; shadow = shadow + 1; if (shadow !== 222) { throw "nearest 181"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker181 = marker + 181; if (marker181 !== 1181) { throw "marker 181"; } score = score + marker181; }
{ let shadow = 222; shadow = shadow + 1; if (shadow !== 223) { throw "nearest 182"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker182 = marker + 182; if (marker182 !== 1182) { throw "marker 182"; } score = score + marker182; }
{ let shadow = 223; shadow = shadow + 1; if (shadow !== 224) { throw "nearest 183"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker183 = marker + 183; if (marker183 !== 1183) { throw "marker 183"; } score = score + marker183; }
{ let shadow = 224; shadow = shadow + 1; if (shadow !== 225) { throw "nearest 184"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker184 = marker + 184; if (marker184 !== 1184) { throw "marker 184"; } score = score + marker184; }
{ let shadow = 225; shadow = shadow + 1; if (shadow !== 226) { throw "nearest 185"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker185 = marker + 185; if (marker185 !== 1185) { throw "marker 185"; } score = score + marker185; }
{ let shadow = 226; shadow = shadow + 1; if (shadow !== 227) { throw "nearest 186"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker186 = marker + 186; if (marker186 !== 1186) { throw "marker 186"; } score = score + marker186; }
{ let shadow = 227; shadow = shadow + 1; if (shadow !== 228) { throw "nearest 187"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker187 = marker + 187; if (marker187 !== 1187) { throw "marker 187"; } score = score + marker187; }
{ let shadow = 228; shadow = shadow + 1; if (shadow !== 229) { throw "nearest 188"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker188 = marker + 188; if (marker188 !== 1188) { throw "marker 188"; } score = score + marker188; }
{ let shadow = 229; shadow = shadow + 1; if (shadow !== 230) { throw "nearest 189"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker189 = marker + 189; if (marker189 !== 1189) { throw "marker 189"; } score = score + marker189; }
{ let shadow = 230; shadow = shadow + 1; if (shadow !== 231) { throw "nearest 190"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker190 = marker + 190; if (marker190 !== 1190) { throw "marker 190"; } score = score + marker190; }
function nested190(shadow) { shadow = shadow + 191; return shadow; }
if (nested190(230) !== 421) { throw "nested 190"; }
{ let shadow = 231; shadow = shadow + 1; if (shadow !== 232) { throw "nearest 191"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker191 = marker + 191; if (marker191 !== 1191) { throw "marker 191"; } score = score + marker191; }
{ let shadow = 232; shadow = shadow + 1; if (shadow !== 233) { throw "nearest 192"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker192 = marker + 192; if (marker192 !== 1192) { throw "marker 192"; } score = score + marker192; }
{ let shadow = 233; shadow = shadow + 1; if (shadow !== 234) { throw "nearest 193"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker193 = marker + 193; if (marker193 !== 1193) { throw "marker 193"; } score = score + marker193; }
{ let shadow = 234; shadow = shadow + 1; if (shadow !== 235) { throw "nearest 194"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker194 = marker + 194; if (marker194 !== 1194) { throw "marker 194"; } score = score + marker194; }
{ let shadow = 235; shadow = shadow + 1; if (shadow !== 236) { throw "nearest 195"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker195 = marker + 195; if (marker195 !== 1195) { throw "marker 195"; } score = score + marker195; }
{ let shadow = 236; shadow = shadow + 1; if (shadow !== 237) { throw "nearest 196"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker196 = marker + 196; if (marker196 !== 1196) { throw "marker 196"; } score = score + marker196; }
{ let shadow = 237; shadow = shadow + 1; if (shadow !== 238) { throw "nearest 197"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker197 = marker + 197; if (marker197 !== 1197) { throw "marker 197"; } score = score + marker197; }
{ let shadow = 238; shadow = shadow + 1; if (shadow !== 239) { throw "nearest 198"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker198 = marker + 198; if (marker198 !== 1198) { throw "marker 198"; } score = score + marker198; }
{ let shadow = 239; shadow = shadow + 1; if (shadow !== 240) { throw "nearest 199"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker199 = marker + 199; if (marker199 !== 1199) { throw "marker 199"; } score = score + marker199; }
{ let shadow = 240; shadow = shadow + 1; if (shadow !== 241) { throw "nearest 200"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker200 = marker + 200; if (marker200 !== 1200) { throw "marker 200"; } score = score + marker200; }
{ let shadow = 241; shadow = shadow + 1; if (shadow !== 242) { throw "nearest 201"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker201 = marker + 201; if (marker201 !== 1201) { throw "marker 201"; } score = score + marker201; }
{ let shadow = 242; shadow = shadow + 1; if (shadow !== 243) { throw "nearest 202"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker202 = marker + 202; if (marker202 !== 1202) { throw "marker 202"; } score = score + marker202; }
{ let shadow = 243; shadow = shadow + 1; if (shadow !== 244) { throw "nearest 203"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker203 = marker + 203; if (marker203 !== 1203) { throw "marker 203"; } score = score + marker203; }
{ let shadow = 244; shadow = shadow + 1; if (shadow !== 245) { throw "nearest 204"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker204 = marker + 204; if (marker204 !== 1204) { throw "marker 204"; } score = score + marker204; }
{ let shadow = 245; shadow = shadow + 1; if (shadow !== 246) { throw "nearest 205"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker205 = marker + 205; if (marker205 !== 1205) { throw "marker 205"; } score = score + marker205; }
{ let shadow = 246; shadow = shadow + 1; if (shadow !== 247) { throw "nearest 206"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker206 = marker + 206; if (marker206 !== 1206) { throw "marker 206"; } score = score + marker206; }
{ let shadow = 247; shadow = shadow + 1; if (shadow !== 248) { throw "nearest 207"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker207 = marker + 207; if (marker207 !== 1207) { throw "marker 207"; } score = score + marker207; }
{ let shadow = 248; shadow = shadow + 1; if (shadow !== 249) { throw "nearest 208"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker208 = marker + 208; if (marker208 !== 1208) { throw "marker 208"; } score = score + marker208; }
{ let shadow = 249; shadow = shadow + 1; if (shadow !== 250) { throw "nearest 209"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker209 = marker + 209; if (marker209 !== 1209) { throw "marker 209"; } score = score + marker209; }
function nested209(shadow) { shadow = shadow + 210; return shadow; }
if (nested209(249) !== 459) { throw "nested 209"; }
{ let shadow = 250; shadow = shadow + 1; if (shadow !== 251) { throw "nearest 210"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker210 = marker + 210; if (marker210 !== 1210) { throw "marker 210"; } score = score + marker210; }
{ let shadow = 251; shadow = shadow + 1; if (shadow !== 252) { throw "nearest 211"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker211 = marker + 211; if (marker211 !== 1211) { throw "marker 211"; } score = score + marker211; }
{ let shadow = 252; shadow = shadow + 1; if (shadow !== 253) { throw "nearest 212"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker212 = marker + 212; if (marker212 !== 1212) { throw "marker 212"; } score = score + marker212; }
{ let shadow = 253; shadow = shadow + 1; if (shadow !== 254) { throw "nearest 213"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker213 = marker + 213; if (marker213 !== 1213) { throw "marker 213"; } score = score + marker213; }
{ let shadow = 254; shadow = shadow + 1; if (shadow !== 255) { throw "nearest 214"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker214 = marker + 214; if (marker214 !== 1214) { throw "marker 214"; } score = score + marker214; }
{ let shadow = 255; shadow = shadow + 1; if (shadow !== 256) { throw "nearest 215"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker215 = marker + 215; if (marker215 !== 1215) { throw "marker 215"; } score = score + marker215; }
{ let shadow = 256; shadow = shadow + 1; if (shadow !== 257) { throw "nearest 216"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker216 = marker + 216; if (marker216 !== 1216) { throw "marker 216"; } score = score + marker216; }
{ let shadow = 257; shadow = shadow + 1; if (shadow !== 258) { throw "nearest 217"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker217 = marker + 217; if (marker217 !== 1217) { throw "marker 217"; } score = score + marker217; }
{ let shadow = 258; shadow = shadow + 1; if (shadow !== 259) { throw "nearest 218"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker218 = marker + 218; if (marker218 !== 1218) { throw "marker 218"; } score = score + marker218; }
{ let shadow = 259; shadow = shadow + 1; if (shadow !== 260) { throw "nearest 219"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker219 = marker + 219; if (marker219 !== 1219) { throw "marker 219"; } score = score + marker219; }
{ let shadow = 260; shadow = shadow + 1; if (shadow !== 261) { throw "nearest 220"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker220 = marker + 220; if (marker220 !== 1220) { throw "marker 220"; } score = score + marker220; }
{ let shadow = 261; shadow = shadow + 1; if (shadow !== 262) { throw "nearest 221"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker221 = marker + 221; if (marker221 !== 1221) { throw "marker 221"; } score = score + marker221; }
{ let shadow = 262; shadow = shadow + 1; if (shadow !== 263) { throw "nearest 222"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker222 = marker + 222; if (marker222 !== 1222) { throw "marker 222"; } score = score + marker222; }
{ let shadow = 263; shadow = shadow + 1; if (shadow !== 264) { throw "nearest 223"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker223 = marker + 223; if (marker223 !== 1223) { throw "marker 223"; } score = score + marker223; }
{ let shadow = 264; shadow = shadow + 1; if (shadow !== 265) { throw "nearest 224"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker224 = marker + 224; if (marker224 !== 1224) { throw "marker 224"; } score = score + marker224; }
{ let shadow = 265; shadow = shadow + 1; if (shadow !== 266) { throw "nearest 225"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker225 = marker + 225; if (marker225 !== 1225) { throw "marker 225"; } score = score + marker225; }
{ let shadow = 266; shadow = shadow + 1; if (shadow !== 267) { throw "nearest 226"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker226 = marker + 226; if (marker226 !== 1226) { throw "marker 226"; } score = score + marker226; }
{ let shadow = 267; shadow = shadow + 1; if (shadow !== 268) { throw "nearest 227"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker227 = marker + 227; if (marker227 !== 1227) { throw "marker 227"; } score = score + marker227; }
{ let shadow = 268; shadow = shadow + 1; if (shadow !== 269) { throw "nearest 228"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker228 = marker + 228; if (marker228 !== 1228) { throw "marker 228"; } score = score + marker228; }
function nested228(shadow) { shadow = shadow + 229; return shadow; }
if (nested228(268) !== 497) { throw "nested 228"; }
{ let shadow = 269; shadow = shadow + 1; if (shadow !== 270) { throw "nearest 229"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker229 = marker + 229; if (marker229 !== 1229) { throw "marker 229"; } score = score + marker229; }
{ let shadow = 270; shadow = shadow + 1; if (shadow !== 271) { throw "nearest 230"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker230 = marker + 230; if (marker230 !== 1230) { throw "marker 230"; } score = score + marker230; }
{ let shadow = 271; shadow = shadow + 1; if (shadow !== 272) { throw "nearest 231"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker231 = marker + 231; if (marker231 !== 1231) { throw "marker 231"; } score = score + marker231; }
{ let shadow = 272; shadow = shadow + 1; if (shadow !== 273) { throw "nearest 232"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker232 = marker + 232; if (marker232 !== 1232) { throw "marker 232"; } score = score + marker232; }
{ let shadow = 273; shadow = shadow + 1; if (shadow !== 274) { throw "nearest 233"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker233 = marker + 233; if (marker233 !== 1233) { throw "marker 233"; } score = score + marker233; }
{ let shadow = 274; shadow = shadow + 1; if (shadow !== 275) { throw "nearest 234"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker234 = marker + 234; if (marker234 !== 1234) { throw "marker 234"; } score = score + marker234; }
{ let shadow = 275; shadow = shadow + 1; if (shadow !== 276) { throw "nearest 235"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker235 = marker + 235; if (marker235 !== 1235) { throw "marker 235"; } score = score + marker235; }
{ let shadow = 276; shadow = shadow + 1; if (shadow !== 277) { throw "nearest 236"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker236 = marker + 236; if (marker236 !== 1236) { throw "marker 236"; } score = score + marker236; }
{ let shadow = 277; shadow = shadow + 1; if (shadow !== 278) { throw "nearest 237"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker237 = marker + 237; if (marker237 !== 1237) { throw "marker 237"; } score = score + marker237; }
{ let shadow = 278; shadow = shadow + 1; if (shadow !== 279) { throw "nearest 238"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker238 = marker + 238; if (marker238 !== 1238) { throw "marker 238"; } score = score + marker238; }
{ let shadow = 279; shadow = shadow + 1; if (shadow !== 280) { throw "nearest 239"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker239 = marker + 239; if (marker239 !== 1239) { throw "marker 239"; } score = score + marker239; }
{ let shadow = 280; shadow = shadow + 1; if (shadow !== 281) { throw "nearest 240"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker240 = marker + 240; if (marker240 !== 1240) { throw "marker 240"; } score = score + marker240; }
{ let shadow = 281; shadow = shadow + 1; if (shadow !== 282) { throw "nearest 241"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker241 = marker + 241; if (marker241 !== 1241) { throw "marker 241"; } score = score + marker241; }
{ let shadow = 282; shadow = shadow + 1; if (shadow !== 283) { throw "nearest 242"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker242 = marker + 242; if (marker242 !== 1242) { throw "marker 242"; } score = score + marker242; }
{ let shadow = 283; shadow = shadow + 1; if (shadow !== 284) { throw "nearest 243"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker243 = marker + 243; if (marker243 !== 1243) { throw "marker 243"; } score = score + marker243; }
{ let shadow = 284; shadow = shadow + 1; if (shadow !== 285) { throw "nearest 244"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker244 = marker + 244; if (marker244 !== 1244) { throw "marker 244"; } score = score + marker244; }
{ let shadow = 285; shadow = shadow + 1; if (shadow !== 286) { throw "nearest 245"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker245 = marker + 245; if (marker245 !== 1245) { throw "marker 245"; } score = score + marker245; }
{ let shadow = 286; shadow = shadow + 1; if (shadow !== 287) { throw "nearest 246"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker246 = marker + 246; if (marker246 !== 1246) { throw "marker 246"; } score = score + marker246; }
{ let shadow = 287; shadow = shadow + 1; if (shadow !== 288) { throw "nearest 247"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker247 = marker + 247; if (marker247 !== 1247) { throw "marker 247"; } score = score + marker247; }
function nested247(shadow) { shadow = shadow + 248; return shadow; }
if (nested247(287) !== 535) { throw "nested 247"; }
{ let shadow = 288; shadow = shadow + 1; if (shadow !== 289) { throw "nearest 248"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker248 = marker + 248; if (marker248 !== 1248) { throw "marker 248"; } score = score + marker248; }
{ let shadow = 289; shadow = shadow + 1; if (shadow !== 290) { throw "nearest 249"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker249 = marker + 249; if (marker249 !== 1249) { throw "marker 249"; } score = score + marker249; }
{ let shadow = 290; shadow = shadow + 1; if (shadow !== 291) { throw "nearest 250"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker250 = marker + 250; if (marker250 !== 1250) { throw "marker 250"; } score = score + marker250; }
{ let shadow = 291; shadow = shadow + 1; if (shadow !== 292) { throw "nearest 251"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker251 = marker + 251; if (marker251 !== 1251) { throw "marker 251"; } score = score + marker251; }
{ let shadow = 292; shadow = shadow + 1; if (shadow !== 293) { throw "nearest 252"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker252 = marker + 252; if (marker252 !== 1252) { throw "marker 252"; } score = score + marker252; }
{ let shadow = 293; shadow = shadow + 1; if (shadow !== 294) { throw "nearest 253"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker253 = marker + 253; if (marker253 !== 1253) { throw "marker 253"; } score = score + marker253; }
{ let shadow = 294; shadow = shadow + 1; if (shadow !== 295) { throw "nearest 254"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker254 = marker + 254; if (marker254 !== 1254) { throw "marker 254"; } score = score + marker254; }
{ let shadow = 295; shadow = shadow + 1; if (shadow !== 296) { throw "nearest 255"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker255 = marker + 255; if (marker255 !== 1255) { throw "marker 255"; } score = score + marker255; }
{ let shadow = 296; shadow = shadow + 1; if (shadow !== 297) { throw "nearest 256"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker256 = marker + 256; if (marker256 !== 1256) { throw "marker 256"; } score = score + marker256; }
{ let shadow = 297; shadow = shadow + 1; if (shadow !== 298) { throw "nearest 257"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker257 = marker + 257; if (marker257 !== 1257) { throw "marker 257"; } score = score + marker257; }
{ let shadow = 298; shadow = shadow + 1; if (shadow !== 299) { throw "nearest 258"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker258 = marker + 258; if (marker258 !== 1258) { throw "marker 258"; } score = score + marker258; }
{ let shadow = 299; shadow = shadow + 1; if (shadow !== 300) { throw "nearest 259"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker259 = marker + 259; if (marker259 !== 1259) { throw "marker 259"; } score = score + marker259; }
{ let shadow = 300; shadow = shadow + 1; if (shadow !== 301) { throw "nearest 260"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker260 = marker + 260; if (marker260 !== 1260) { throw "marker 260"; } score = score + marker260; }
{ let shadow = 301; shadow = shadow + 1; if (shadow !== 302) { throw "nearest 261"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker261 = marker + 261; if (marker261 !== 1261) { throw "marker 261"; } score = score + marker261; }
{ let shadow = 302; shadow = shadow + 1; if (shadow !== 303) { throw "nearest 262"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker262 = marker + 262; if (marker262 !== 1262) { throw "marker 262"; } score = score + marker262; }
{ let shadow = 303; shadow = shadow + 1; if (shadow !== 304) { throw "nearest 263"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker263 = marker + 263; if (marker263 !== 1263) { throw "marker 263"; } score = score + marker263; }
{ let shadow = 304; shadow = shadow + 1; if (shadow !== 305) { throw "nearest 264"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker264 = marker + 264; if (marker264 !== 1264) { throw "marker 264"; } score = score + marker264; }
{ let shadow = 305; shadow = shadow + 1; if (shadow !== 306) { throw "nearest 265"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker265 = marker + 265; if (marker265 !== 1265) { throw "marker 265"; } score = score + marker265; }
{ let shadow = 306; shadow = shadow + 1; if (shadow !== 307) { throw "nearest 266"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker266 = marker + 266; if (marker266 !== 1266) { throw "marker 266"; } score = score + marker266; }
function nested266(shadow) { shadow = shadow + 267; return shadow; }
if (nested266(306) !== 573) { throw "nested 266"; }
{ let shadow = 307; shadow = shadow + 1; if (shadow !== 308) { throw "nearest 267"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker267 = marker + 267; if (marker267 !== 1267) { throw "marker 267"; } score = score + marker267; }
{ let shadow = 308; shadow = shadow + 1; if (shadow !== 309) { throw "nearest 268"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker268 = marker + 268; if (marker268 !== 1268) { throw "marker 268"; } score = score + marker268; }
{ let shadow = 309; shadow = shadow + 1; if (shadow !== 310) { throw "nearest 269"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker269 = marker + 269; if (marker269 !== 1269) { throw "marker 269"; } score = score + marker269; }
{ let shadow = 310; shadow = shadow + 1; if (shadow !== 311) { throw "nearest 270"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker270 = marker + 270; if (marker270 !== 1270) { throw "marker 270"; } score = score + marker270; }
{ let shadow = 311; shadow = shadow + 1; if (shadow !== 312) { throw "nearest 271"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker271 = marker + 271; if (marker271 !== 1271) { throw "marker 271"; } score = score + marker271; }
{ let shadow = 312; shadow = shadow + 1; if (shadow !== 313) { throw "nearest 272"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker272 = marker + 272; if (marker272 !== 1272) { throw "marker 272"; } score = score + marker272; }
{ let shadow = 313; shadow = shadow + 1; if (shadow !== 314) { throw "nearest 273"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker273 = marker + 273; if (marker273 !== 1273) { throw "marker 273"; } score = score + marker273; }
{ let shadow = 314; shadow = shadow + 1; if (shadow !== 315) { throw "nearest 274"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker274 = marker + 274; if (marker274 !== 1274) { throw "marker 274"; } score = score + marker274; }
{ let shadow = 315; shadow = shadow + 1; if (shadow !== 316) { throw "nearest 275"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker275 = marker + 275; if (marker275 !== 1275) { throw "marker 275"; } score = score + marker275; }
{ let shadow = 316; shadow = shadow + 1; if (shadow !== 317) { throw "nearest 276"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker276 = marker + 276; if (marker276 !== 1276) { throw "marker 276"; } score = score + marker276; }
{ let shadow = 317; shadow = shadow + 1; if (shadow !== 318) { throw "nearest 277"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker277 = marker + 277; if (marker277 !== 1277) { throw "marker 277"; } score = score + marker277; }
{ let shadow = 318; shadow = shadow + 1; if (shadow !== 319) { throw "nearest 278"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker278 = marker + 278; if (marker278 !== 1278) { throw "marker 278"; } score = score + marker278; }
{ let shadow = 319; shadow = shadow + 1; if (shadow !== 320) { throw "nearest 279"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker279 = marker + 279; if (marker279 !== 1279) { throw "marker 279"; } score = score + marker279; }
{ let shadow = 320; shadow = shadow + 1; if (shadow !== 321) { throw "nearest 280"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker280 = marker + 280; if (marker280 !== 1280) { throw "marker 280"; } score = score + marker280; }
{ let shadow = 321; shadow = shadow + 1; if (shadow !== 322) { throw "nearest 281"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker281 = marker + 281; if (marker281 !== 1281) { throw "marker 281"; } score = score + marker281; }
{ let shadow = 322; shadow = shadow + 1; if (shadow !== 323) { throw "nearest 282"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker282 = marker + 282; if (marker282 !== 1282) { throw "marker 282"; } score = score + marker282; }
{ let shadow = 323; shadow = shadow + 1; if (shadow !== 324) { throw "nearest 283"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker283 = marker + 283; if (marker283 !== 1283) { throw "marker 283"; } score = score + marker283; }
{ let shadow = 324; shadow = shadow + 1; if (shadow !== 325) { throw "nearest 284"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker284 = marker + 284; if (marker284 !== 1284) { throw "marker 284"; } score = score + marker284; }
{ let shadow = 325; shadow = shadow + 1; if (shadow !== 326) { throw "nearest 285"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker285 = marker + 285; if (marker285 !== 1285) { throw "marker 285"; } score = score + marker285; }
function nested285(shadow) { shadow = shadow + 286; return shadow; }
if (nested285(325) !== 611) { throw "nested 285"; }
{ let shadow = 326; shadow = shadow + 1; if (shadow !== 327) { throw "nearest 286"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker286 = marker + 286; if (marker286 !== 1286) { throw "marker 286"; } score = score + marker286; }
{ let shadow = 327; shadow = shadow + 1; if (shadow !== 328) { throw "nearest 287"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker287 = marker + 287; if (marker287 !== 1287) { throw "marker 287"; } score = score + marker287; }
{ let shadow = 328; shadow = shadow + 1; if (shadow !== 329) { throw "nearest 288"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker288 = marker + 288; if (marker288 !== 1288) { throw "marker 288"; } score = score + marker288; }
{ let shadow = 329; shadow = shadow + 1; if (shadow !== 330) { throw "nearest 289"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker289 = marker + 289; if (marker289 !== 1289) { throw "marker 289"; } score = score + marker289; }
{ let shadow = 330; shadow = shadow + 1; if (shadow !== 331) { throw "nearest 290"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker290 = marker + 290; if (marker290 !== 1290) { throw "marker 290"; } score = score + marker290; }
{ let shadow = 331; shadow = shadow + 1; if (shadow !== 332) { throw "nearest 291"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker291 = marker + 291; if (marker291 !== 1291) { throw "marker 291"; } score = score + marker291; }
{ let shadow = 332; shadow = shadow + 1; if (shadow !== 333) { throw "nearest 292"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker292 = marker + 292; if (marker292 !== 1292) { throw "marker 292"; } score = score + marker292; }
{ let shadow = 333; shadow = shadow + 1; if (shadow !== 334) { throw "nearest 293"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker293 = marker + 293; if (marker293 !== 1293) { throw "marker 293"; } score = score + marker293; }
{ let shadow = 334; shadow = shadow + 1; if (shadow !== 335) { throw "nearest 294"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker294 = marker + 294; if (marker294 !== 1294) { throw "marker 294"; } score = score + marker294; }
{ let shadow = 335; shadow = shadow + 1; if (shadow !== 336) { throw "nearest 295"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker295 = marker + 295; if (marker295 !== 1295) { throw "marker 295"; } score = score + marker295; }
{ let shadow = 336; shadow = shadow + 1; if (shadow !== 337) { throw "nearest 296"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker296 = marker + 296; if (marker296 !== 1296) { throw "marker 296"; } score = score + marker296; }
{ let shadow = 337; shadow = shadow + 1; if (shadow !== 338) { throw "nearest 297"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker297 = marker + 297; if (marker297 !== 1297) { throw "marker 297"; } score = score + marker297; }
{ let shadow = 338; shadow = shadow + 1; if (shadow !== 339) { throw "nearest 298"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker298 = marker + 298; if (marker298 !== 1298) { throw "marker 298"; } score = score + marker298; }
{ let shadow = 339; shadow = shadow + 1; if (shadow !== 340) { throw "nearest 299"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker299 = marker + 299; if (marker299 !== 1299) { throw "marker 299"; } score = score + marker299; }
{ let shadow = 340; shadow = shadow + 1; if (shadow !== 341) { throw "nearest 300"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker300 = marker + 300; if (marker300 !== 1300) { throw "marker 300"; } score = score + marker300; }
{ let shadow = 341; shadow = shadow + 1; if (shadow !== 342) { throw "nearest 301"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker301 = marker + 301; if (marker301 !== 1301) { throw "marker 301"; } score = score + marker301; }
{ let shadow = 342; shadow = shadow + 1; if (shadow !== 343) { throw "nearest 302"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker302 = marker + 302; if (marker302 !== 1302) { throw "marker 302"; } score = score + marker302; }
{ let shadow = 343; shadow = shadow + 1; if (shadow !== 344) { throw "nearest 303"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker303 = marker + 303; if (marker303 !== 1303) { throw "marker 303"; } score = score + marker303; }
{ let shadow = 344; shadow = shadow + 1; if (shadow !== 345) { throw "nearest 304"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker304 = marker + 304; if (marker304 !== 1304) { throw "marker 304"; } score = score + marker304; }
function nested304(shadow) { shadow = shadow + 305; return shadow; }
if (nested304(344) !== 649) { throw "nested 304"; }
{ let shadow = 345; shadow = shadow + 1; if (shadow !== 346) { throw "nearest 305"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker305 = marker + 305; if (marker305 !== 1305) { throw "marker 305"; } score = score + marker305; }
{ let shadow = 346; shadow = shadow + 1; if (shadow !== 347) { throw "nearest 306"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker306 = marker + 306; if (marker306 !== 1306) { throw "marker 306"; } score = score + marker306; }
{ let shadow = 347; shadow = shadow + 1; if (shadow !== 348) { throw "nearest 307"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker307 = marker + 307; if (marker307 !== 1307) { throw "marker 307"; } score = score + marker307; }
{ let shadow = 348; shadow = shadow + 1; if (shadow !== 349) { throw "nearest 308"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker308 = marker + 308; if (marker308 !== 1308) { throw "marker 308"; } score = score + marker308; }
{ let shadow = 349; shadow = shadow + 1; if (shadow !== 350) { throw "nearest 309"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker309 = marker + 309; if (marker309 !== 1309) { throw "marker 309"; } score = score + marker309; }
{ let shadow = 350; shadow = shadow + 1; if (shadow !== 351) { throw "nearest 310"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker310 = marker + 310; if (marker310 !== 1310) { throw "marker 310"; } score = score + marker310; }
{ let shadow = 351; shadow = shadow + 1; if (shadow !== 352) { throw "nearest 311"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker311 = marker + 311; if (marker311 !== 1311) { throw "marker 311"; } score = score + marker311; }
{ let shadow = 352; shadow = shadow + 1; if (shadow !== 353) { throw "nearest 312"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker312 = marker + 312; if (marker312 !== 1312) { throw "marker 312"; } score = score + marker312; }
{ let shadow = 353; shadow = shadow + 1; if (shadow !== 354) { throw "nearest 313"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker313 = marker + 313; if (marker313 !== 1313) { throw "marker 313"; } score = score + marker313; }
{ let shadow = 354; shadow = shadow + 1; if (shadow !== 355) { throw "nearest 314"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker314 = marker + 314; if (marker314 !== 1314) { throw "marker 314"; } score = score + marker314; }
{ let shadow = 355; shadow = shadow + 1; if (shadow !== 356) { throw "nearest 315"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker315 = marker + 315; if (marker315 !== 1315) { throw "marker 315"; } score = score + marker315; }
{ let shadow = 356; shadow = shadow + 1; if (shadow !== 357) { throw "nearest 316"; } score = score + shadow; }
if (shadow !== 10) { throw "outer preserved"; }
{ var marker316 = marker + 316; if (marker316 !== 1316) { throw "marker 316"; } score = score + marker316; }
return score;
}
console.log("ok", __ayyRun(), shadow, marker);
