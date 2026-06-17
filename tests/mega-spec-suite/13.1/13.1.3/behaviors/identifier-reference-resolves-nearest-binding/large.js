// behavior: identifier-reference-resolves-nearest-binding
// expected: pass
// goal: script
// size: large
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
return score;
}
console.log("ok", __ayyRun(), shadow, marker);
