// behavior: private-identifier-prefix
// expected: pass
// goal: script
// size: large
// variant: script.strict

class __AyyPrivateBox { #value; constructor(value) { this.#value = value; } getValue() { return this.#value; } setValue(value) { this.#value = value; } }
function __ayyRun() {
var privateBox = new __AyyPrivateBox(31);
if (privateBox.getValue() !== 31) { throw "private-get"; }
privateBox.setValue(37);
if (privateBox.getValue() !== 37) { throw "private-set"; }
var v0 = 0;
if (v0 !== 0) { throw "v0"; }
if ((v0 + 1) !== 1) { throw "branch0"; }
var v1 = 1;
if (v1 !== 1) { throw "v1"; }
var v2 = 2;
if (v2 !== 2) { throw "v2"; }
var v3 = 3;
if (v3 !== 3) { throw "v3"; }
var v4 = 4;
if (v4 !== 4) { throw "v4"; }
var v5 = 5;
if (v5 !== 5) { throw "v5"; }
var v6 = 6;
if (v6 !== 6) { throw "v6"; }
var v7 = 7;
if (v7 !== 7) { throw "v7"; }
var v8 = 8;
if (v8 !== 8) { throw "v8"; }
var v9 = 9;
if (v9 !== 9) { throw "v9"; }
var v10 = 10;
if (v10 !== 10) { throw "v10"; }
var v11 = 11;
if (v11 !== 11) { throw "v11"; }
var v12 = 12;
if (v12 !== 12) { throw "v12"; }
var v13 = 13;
if (v13 !== 13) { throw "v13"; }
var v14 = 14;
if (v14 !== 14) { throw "v14"; }
var v15 = 15;
if (v15 !== 15) { throw "v15"; }
var v16 = 16;
if (v16 !== 16) { throw "v16"; }
var v17 = 17;
if (v17 !== 17) { throw "v17"; }
if ((v17 + 1) !== 18) { throw "branch17"; }
var v18 = 18;
if (v18 !== 18) { throw "v18"; }
var v19 = 19;
if (v19 !== 19) { throw "v19"; }
var v20 = 20;
if (v20 !== 20) { throw "v20"; }
var v21 = 21;
if (v21 !== 21) { throw "v21"; }
var v22 = 22;
if (v22 !== 22) { throw "v22"; }
var v23 = 23;
if (v23 !== 23) { throw "v23"; }
var v24 = 24;
if (v24 !== 24) { throw "v24"; }
var v25 = 25;
if (v25 !== 25) { throw "v25"; }
var v26 = 26;
if (v26 !== 26) { throw "v26"; }
var v27 = 27;
if (v27 !== 27) { throw "v27"; }
var v28 = 28;
if (v28 !== 28) { throw "v28"; }
var v29 = 29;
if (v29 !== 29) { throw "v29"; }
var v30 = 30;
if (v30 !== 30) { throw "v30"; }
var v31 = 31;
if (v31 !== 31) { throw "v31"; }
var v32 = 32;
if (v32 !== 32) { throw "v32"; }
var v33 = 33;
if (v33 !== 33) { throw "v33"; }
var v34 = 34;
if (v34 !== 34) { throw "v34"; }
if ((v34 + 1) !== 35) { throw "branch34"; }
var v35 = 35;
if (v35 !== 35) { throw "v35"; }
var v36 = 36;
if (v36 !== 36) { throw "v36"; }
var v37 = 37;
if (v37 !== 37) { throw "v37"; }
var v38 = 38;
if (v38 !== 38) { throw "v38"; }
var v39 = 39;
if (v39 !== 39) { throw "v39"; }
var v40 = 40;
if (v40 !== 40) { throw "v40"; }
var v41 = 41;
if (v41 !== 41) { throw "v41"; }
var v42 = 42;
if (v42 !== 42) { throw "v42"; }
var v43 = 43;
if (v43 !== 43) { throw "v43"; }
var v44 = 44;
if (v44 !== 44) { throw "v44"; }
var v45 = 45;
if (v45 !== 45) { throw "v45"; }
var v46 = 46;
if (v46 !== 46) { throw "v46"; }
var v47 = 47;
if (v47 !== 47) { throw "v47"; }
var v48 = 48;
if (v48 !== 48) { throw "v48"; }
var v49 = 49;
if (v49 !== 49) { throw "v49"; }
var v50 = 50;
if (v50 !== 50) { throw "v50"; }
var v51 = 51;
if (v51 !== 51) { throw "v51"; }
if ((v51 + 1) !== 52) { throw "branch51"; }
var v52 = 52;
if (v52 !== 52) { throw "v52"; }
var v53 = 53;
if (v53 !== 53) { throw "v53"; }
var v54 = 54;
if (v54 !== 54) { throw "v54"; }
var v55 = 55;
if (v55 !== 55) { throw "v55"; }
var v56 = 56;
if (v56 !== 56) { throw "v56"; }
var v57 = 57;
if (v57 !== 57) { throw "v57"; }
var v58 = 58;
if (v58 !== 58) { throw "v58"; }
var v59 = 59;
if (v59 !== 59) { throw "v59"; }
var v60 = 60;
if (v60 !== 60) { throw "v60"; }
var v61 = 61;
if (v61 !== 61) { throw "v61"; }
var v62 = 62;
if (v62 !== 62) { throw "v62"; }
var v63 = 63;
if (v63 !== 63) { throw "v63"; }
var v64 = 64;
if (v64 !== 64) { throw "v64"; }
var v65 = 65;
if (v65 !== 65) { throw "v65"; }
var v66 = 66;
if (v66 !== 66) { throw "v66"; }
var v67 = 67;
if (v67 !== 67) { throw "v67"; }
var v68 = 68;
if (v68 !== 68) { throw "v68"; }
if ((v68 + 1) !== 69) { throw "branch68"; }
var v69 = 69;
if (v69 !== 69) { throw "v69"; }
var v70 = 70;
if (v70 !== 70) { throw "v70"; }
var v71 = 71;
if (v71 !== 71) { throw "v71"; }
var v72 = 72;
if (v72 !== 72) { throw "v72"; }
var v73 = 73;
if (v73 !== 73) { throw "v73"; }
var v74 = 74;
if (v74 !== 74) { throw "v74"; }
var v75 = 75;
if (v75 !== 75) { throw "v75"; }
var v76 = 76;
if (v76 !== 76) { throw "v76"; }
var v77 = 77;
if (v77 !== 77) { throw "v77"; }
var v78 = 78;
if (v78 !== 78) { throw "v78"; }
var v79 = 79;
if (v79 !== 79) { throw "v79"; }
var v80 = 80;
if (v80 !== 80) { throw "v80"; }
var v81 = 81;
if (v81 !== 81) { throw "v81"; }
var v82 = 82;
if (v82 !== 82) { throw "v82"; }
var v83 = 83;
if (v83 !== 83) { throw "v83"; }
var v84 = 84;
if (v84 !== 84) { throw "v84"; }
var v85 = 85;
if (v85 !== 85) { throw "v85"; }
if ((v85 + 1) !== 86) { throw "branch85"; }
var v86 = 86;
if (v86 !== 86) { throw "v86"; }
var v87 = 87;
if (v87 !== 87) { throw "v87"; }
var v88 = 88;
if (v88 !== 88) { throw "v88"; }
var v89 = 89;
if (v89 !== 89) { throw "v89"; }
var v90 = 90;
if (v90 !== 90) { throw "v90"; }
var v91 = 91;
if (v91 !== 91) { throw "v91"; }
var v92 = 92;
if (v92 !== 92) { throw "v92"; }
var v93 = 93;
if (v93 !== 93) { throw "v93"; }
var v94 = 94;
if (v94 !== 94) { throw "v94"; }
var v95 = 95;
if (v95 !== 95) { throw "v95"; }
var v96 = 96;
if (v96 !== 96) { throw "v96"; }
var v97 = 97;
if (v97 !== 97) { throw "v97"; }
var v98 = 98;
if (v98 !== 98) { throw "v98"; }
var v99 = 99;
if (v99 !== 99) { throw "v99"; }
var v100 = 100;
if (v100 !== 100) { throw "v100"; }
var v101 = 101;
if (v101 !== 101) { throw "v101"; }
var v102 = 102;
if (v102 !== 102) { throw "v102"; }
if ((v102 + 1) !== 103) { throw "branch102"; }
var v103 = 103;
if (v103 !== 103) { throw "v103"; }
var v104 = 104;
if (v104 !== 104) { throw "v104"; }
var v105 = 105;
if (v105 !== 105) { throw "v105"; }
var v106 = 106;
if (v106 !== 106) { throw "v106"; }
var v107 = 107;
if (v107 !== 107) { throw "v107"; }
var v108 = 108;
if (v108 !== 108) { throw "v108"; }
var v109 = 109;
if (v109 !== 109) { throw "v109"; }
var v110 = 110;
if (v110 !== 110) { throw "v110"; }
var v111 = 111;
if (v111 !== 111) { throw "v111"; }
var v112 = 112;
if (v112 !== 112) { throw "v112"; }
var v113 = 113;
if (v113 !== 113) { throw "v113"; }
var v114 = 114;
if (v114 !== 114) { throw "v114"; }
var v115 = 115;
if (v115 !== 115) { throw "v115"; }
var v116 = 116;
if (v116 !== 116) { throw "v116"; }
var v117 = 117;
if (v117 !== 117) { throw "v117"; }
var v118 = 118;
if (v118 !== 118) { throw "v118"; }
var v119 = 119;
if (v119 !== 119) { throw "v119"; }
if ((v119 + 1) !== 120) { throw "branch119"; }
var v120 = 120;
if (v120 !== 120) { throw "v120"; }
var v121 = 121;
if (v121 !== 121) { throw "v121"; }
var v122 = 122;
if (v122 !== 122) { throw "v122"; }
var v123 = 123;
if (v123 !== 123) { throw "v123"; }
var v124 = 124;
if (v124 !== 124) { throw "v124"; }
var v125 = 125;
if (v125 !== 125) { throw "v125"; }
var v126 = 126;
if (v126 !== 126) { throw "v126"; }
var v127 = 127;
if (v127 !== 127) { throw "v127"; }
var v128 = 128;
if (v128 !== 128) { throw "v128"; }
var v129 = 129;
if (v129 !== 129) { throw "v129"; }
var v130 = 130;
if (v130 !== 130) { throw "v130"; }
var v131 = 131;
if (v131 !== 131) { throw "v131"; }
var v132 = 132;
if (v132 !== 132) { throw "v132"; }
var v133 = 133;
if (v133 !== 133) { throw "v133"; }
var v134 = 134;
if (v134 !== 134) { throw "v134"; }
var v135 = 135;
if (v135 !== 135) { throw "v135"; }
var v136 = 136;
if (v136 !== 136) { throw "v136"; }
if ((v136 + 1) !== 137) { throw "branch136"; }
var v137 = 137;
if (v137 !== 137) { throw "v137"; }
var v138 = 138;
if (v138 !== 138) { throw "v138"; }
var v139 = 139;
if (v139 !== 139) { throw "v139"; }
var v140 = 140;
if (v140 !== 140) { throw "v140"; }
var v141 = 141;
if (v141 !== 141) { throw "v141"; }
var v142 = 142;
if (v142 !== 142) { throw "v142"; }
var v143 = 143;
if (v143 !== 143) { throw "v143"; }
var v144 = 144;
if (v144 !== 144) { throw "v144"; }
var v145 = 145;
if (v145 !== 145) { throw "v145"; }
var v146 = 146;
if (v146 !== 146) { throw "v146"; }
var v147 = 147;
if (v147 !== 147) { throw "v147"; }
var v148 = 148;
if (v148 !== 148) { throw "v148"; }
var v149 = 149;
if (v149 !== 149) { throw "v149"; }
var v150 = 150;
if (v150 !== 150) { throw "v150"; }
var v151 = 151;
if (v151 !== 151) { throw "v151"; }
var v152 = 152;
if (v152 !== 152) { throw "v152"; }
var v153 = 153;
if (v153 !== 153) { throw "v153"; }
if ((v153 + 1) !== 154) { throw "branch153"; }
var v154 = 154;
if (v154 !== 154) { throw "v154"; }
var v155 = 155;
if (v155 !== 155) { throw "v155"; }
var v156 = 156;
if (v156 !== 156) { throw "v156"; }
var v157 = 157;
if (v157 !== 157) { throw "v157"; }
var v158 = 158;
if (v158 !== 158) { throw "v158"; }
var v159 = 159;
if (v159 !== 159) { throw "v159"; }
var v160 = 160;
if (v160 !== 160) { throw "v160"; }
var v161 = 161;
if (v161 !== 161) { throw "v161"; }
var v162 = 162;
if (v162 !== 162) { throw "v162"; }
var v163 = 163;
if (v163 !== 163) { throw "v163"; }
var v164 = 164;
if (v164 !== 164) { throw "v164"; }
var v165 = 165;
if (v165 !== 165) { throw "v165"; }
var v166 = 166;
if (v166 !== 166) { throw "v166"; }
var v167 = 167;
if (v167 !== 167) { throw "v167"; }
var v168 = 168;
if (v168 !== 168) { throw "v168"; }
var v169 = 169;
if (v169 !== 169) { throw "v169"; }
var v170 = 170;
if (v170 !== 170) { throw "v170"; }
if ((v170 + 1) !== 171) { throw "branch170"; }
var v171 = 171;
if (v171 !== 171) { throw "v171"; }
var v172 = 172;
if (v172 !== 172) { throw "v172"; }
var v173 = 173;
if (v173 !== 173) { throw "v173"; }
var v174 = 174;
if (v174 !== 174) { throw "v174"; }
var v175 = 175;
if (v175 !== 175) { throw "v175"; }
var v176 = 176;
if (v176 !== 176) { throw "v176"; }
var v177 = 177;
if (v177 !== 177) { throw "v177"; }
var v178 = 178;
if (v178 !== 178) { throw "v178"; }
var v179 = 179;
if (v179 !== 179) { throw "v179"; }
var v180 = 180;
if (v180 !== 180) { throw "v180"; }
var v181 = 181;
if (v181 !== 181) { throw "v181"; }
var v182 = 182;
if (v182 !== 182) { throw "v182"; }
var v183 = 183;
if (v183 !== 183) { throw "v183"; }
var v184 = 184;
if (v184 !== 184) { throw "v184"; }
var v185 = 185;
if (v185 !== 185) { throw "v185"; }
var v186 = 186;
if (v186 !== 186) { throw "v186"; }
var v187 = 187;
if (v187 !== 187) { throw "v187"; }
if ((v187 + 1) !== 188) { throw "branch187"; }
var v188 = 188;
if (v188 !== 188) { throw "v188"; }
var v189 = 189;
if (v189 !== 189) { throw "v189"; }
var v190 = 190;
if (v190 !== 190) { throw "v190"; }
var v191 = 191;
if (v191 !== 191) { throw "v191"; }
var v192 = 192;
if (v192 !== 192) { throw "v192"; }
var v193 = 193;
if (v193 !== 193) { throw "v193"; }
var v194 = 194;
if (v194 !== 194) { throw "v194"; }
var v195 = 195;
if (v195 !== 195) { throw "v195"; }
var v196 = 196;
if (v196 !== 196) { throw "v196"; }
var v197 = 197;
if (v197 !== 197) { throw "v197"; }
var v198 = 198;
if (v198 !== 198) { throw "v198"; }
var v199 = 199;
if (v199 !== 199) { throw "v199"; }
var v200 = 200;
if (v200 !== 200) { throw "v200"; }
var v201 = 201;
if (v201 !== 201) { throw "v201"; }
var v202 = 202;
if (v202 !== 202) { throw "v202"; }
var v203 = 203;
if (v203 !== 203) { throw "v203"; }
var v204 = 204;
if (v204 !== 204) { throw "v204"; }
if ((v204 + 1) !== 205) { throw "branch204"; }
var v205 = 205;
if (v205 !== 205) { throw "v205"; }
var v206 = 206;
if (v206 !== 206) { throw "v206"; }
var v207 = 207;
if (v207 !== 207) { throw "v207"; }
var v208 = 208;
if (v208 !== 208) { throw "v208"; }
var v209 = 209;
if (v209 !== 209) { throw "v209"; }
var v210 = 210;
if (v210 !== 210) { throw "v210"; }
var v211 = 211;
if (v211 !== 211) { throw "v211"; }
var v212 = 212;
if (v212 !== 212) { throw "v212"; }
var v213 = 213;
if (v213 !== 213) { throw "v213"; }
var v214 = 214;
if (v214 !== 214) { throw "v214"; }
var v215 = 215;
if (v215 !== 215) { throw "v215"; }
var v216 = 216;
if (v216 !== 216) { throw "v216"; }
var v217 = 217;
if (v217 !== 217) { throw "v217"; }
var v218 = 218;
if (v218 !== 218) { throw "v218"; }
var v219 = 219;
if (v219 !== 219) { throw "v219"; }
var v220 = 220;
if (v220 !== 220) { throw "v220"; }
var v221 = 221;
if (v221 !== 221) { throw "v221"; }
if ((v221 + 1) !== 222) { throw "branch221"; }
var v222 = 222;
if (v222 !== 222) { throw "v222"; }
var v223 = 223;
if (v223 !== 223) { throw "v223"; }
var v224 = 224;
if (v224 !== 224) { throw "v224"; }
var v225 = 225;
if (v225 !== 225) { throw "v225"; }
var v226 = 226;
if (v226 !== 226) { throw "v226"; }
var v227 = 227;
if (v227 !== 227) { throw "v227"; }
var v228 = 228;
if (v228 !== 228) { throw "v228"; }
var v229 = 229;
if (v229 !== 229) { throw "v229"; }
var v230 = 230;
if (v230 !== 230) { throw "v230"; }
var v231 = 231;
if (v231 !== 231) { throw "v231"; }
var v232 = 232;
if (v232 !== 232) { throw "v232"; }
var v233 = 233;
if (v233 !== 233) { throw "v233"; }
var v234 = 234;
if (v234 !== 234) { throw "v234"; }
var v235 = 235;
if (v235 !== 235) { throw "v235"; }
var v236 = 236;
if (v236 !== 236) { throw "v236"; }
var v237 = 237;
if (v237 !== 237) { throw "v237"; }
var v238 = 238;
if (v238 !== 238) { throw "v238"; }
if ((v238 + 1) !== 239) { throw "branch238"; }
return privateBox.getValue();
}
if (__ayyRun() !== 37) { throw "result"; }
