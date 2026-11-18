"""Aleph reference solutions for HumanEval problems (body after the prompt)."""

S = {}

S[3] = """ belowfrom operations 0
fn belowfrom ops:list i64 bal:i64 -> bool
 ex belowfrom [1 -2] 0 = true
 let h else at ops 0 0
 if == len ops 0 false if < + bal h 0 true belowfrom slice ops 1 len ops + bal h
"""

S[5] = """ interfrom numbers delimeter empty i64
fn interfrom xs:list i64 d:i64 acc:list i64 -> list i64
 ex interfrom [1 2] 0 empty i64 = [1 0 2]
 let h else at xs 0 0
 let rest slice xs 1 len xs
 if == len xs 0 acc if == len rest 0 push acc h interfrom rest d push push acc h d
"""

S[7] = """ filtersub strings substring empty str
fn filtersub xs:list str s:str acc:list str -> list str
 ex filtersub ["ab" "c"] "a" empty str = ["ab"]
 let h else at xs 0 ""
 if == len xs 0 acc filtersub slice xs 1 len xs s if contains h s push acc h acc
"""

S[9] = """ rollfrom numbers empty i64
fn rollfrom xs:list i64 acc:list i64 -> list i64
 ex rollfrom [1 3 2] empty i64 = [1 3 3]
 let h else at xs 0 0
 let m else at acc - len acc 1 h
 if == len xs 0 acc rollfrom slice xs 1 len xs push acc if > h m h m
"""

S[12] = """ if == len strings 0 none str some fold @longer "" strings
fn longer a:str b:str -> str
 ex longer "a" "bb" = "bb"
 if > count b count a b a
"""

S[13] = """ if == b 0 a greatestcommondivisor b % a b
"""

S[14] = """ prefixfrom split string "" 1 empty str
fn prefixfrom cs:list str n:i64 acc:list str -> list str
 ex prefixfrom ["a" "b"] 1 empty str = ["a" "ab"]
 if > n len cs acc prefixfrom cs + n 1 push acc join slice cs 0 n ""
"""

S[15] = """ join map @tostr range 0 + n 1 " "
fn tostr k:i64 -> str
 ex tostr 1 = "1"
 show k
"""

S[16] = """ size fold @mark dict str bool split lower string ""
fn mark seen:dict str bool c:str -> dict str bool
 ex size mark dict str bool "a" = 1
 put seen c true
"""

S[18] = """ timesfrom split string "" split substring "" 0
fn timesfrom cs:list str sub:list str i:i64 -> i64
 ex timesfrom ["a" "a"] ["a"] 0 = 2
 if > + i len sub len cs 0 + if == join slice cs i + i len sub "" join sub "" 1 0 timesfrom cs sub + i 1
"""

S[23] = """ count string
"""

S[24] = """ divisorbelow n - n 1
fn divisorbelow m:i64 i:i64 -> i64
 ex divisorbelow 15 14 = 5
 if == % m i 0 i divisorbelow m - i 1
"""

S[25] = """ factorfrom n 2 empty i64
fn factorfrom m:i64 d:i64 acc:list i64 -> list i64
 ex factorfrom 12 2 empty i64 = [2 2 3]
 if > * d d m if > m 1 push acc m acc if == % m d 0 factorfrom / m d d push acc d factorfrom m + d 1 acc
"""

S[26] = """ keeponce numbers fold @bump dict i64 i64 numbers empty i64
fn bump counts:dict i64 i64 x:i64 -> dict i64 i64
 ex size bump dict i64 i64 1 = 1
 put counts x + else get counts x 0 1
fn keeponce xs:list i64 counts:dict i64 i64 acc:list i64 -> list i64
 ex keeponce [1 2 1] bump bump bump dict i64 i64 1 2 1 empty i64 = [2]
 let h else at xs 0 0
 if == len xs 0 acc keeponce slice xs 1 len xs counts if == else get counts h 0 1 push acc h acc
"""

S[28] = """ join strings ""
"""

S[29] = """ filterpre strings prefix empty str
fn filterpre xs:list str p:str acc:list str -> list str
 ex filterpre ["ab" "b"] "a" empty str = ["ab"]
 let h else at xs 0 ""
 let starts == join slice split h "" 0 count p "" p
 if == len xs 0 acc filterpre slice xs 1 len xs p if starts push acc h acc
"""

S[30] = """ filter @positive l
fn positive x:i64 -> bool
 ex positive 1 = true
 > x 0
"""

S[31] = """ if < n 2 false nodivisor n 2
fn nodivisor m:i64 k:i64 -> bool
 ex nodivisor 7 2 = true
 if > * k k m true if == % m k 0 false nodivisor m + k 1
"""

S[34] = """ keys fold @mark dict i64 bool l
fn mark seen:dict i64 bool x:i64 -> dict i64 bool
 ex size mark dict i64 bool 1 = 1
 put seen x true
"""

S[35] = """ fold @bigger else at l 0 0 l
fn bigger a:i64 b:i64 -> i64
 ex bigger 1 2 = 2
 if > b a b a
"""

S[36] = """ sum map @sevens filter @by1113 range 0 n
fn by1113 i:i64 -> bool
 ex by1113 22 = true
 or == % i 11 0 == % i 13 0
fn isseven c:str -> bool
 ex isseven "7" = true
 == c "7"
fn sevens i:i64 -> i64
 ex sevens 77 = 2
 len filter @isseven split show i ""
"""

S[42] = """ map @inc l
fn inc x:i64 -> i64
 ex inc 1 = 2
 + x 1
"""

S[43] = """ pairfrom l 0
fn hasneg xs:list i64 x:i64 -> bool
 ex hasneg [1 -2] 2 = true
 if == len xs 0 false or == + else at xs 0 0 x 0 hasneg slice xs 1 len xs x
fn pairfrom xs:list i64 i:i64 -> bool
 ex pairfrom [1 -1] 0 = true
 if >= i len xs false or hasneg slice xs + i 1 len xs else at xs i 0 pairfrom xs + i 1
"""

S[44] = """ if < x base show x cat changebase / x base base show % x base
"""

S[46] = """ fib4from n 0 0 2 0
fn fib4from k:i64 a:i64 b:i64 c:i64 d:i64 -> i64
 ex fib4from 0 0 0 2 0 = 0
 if == k 0 a fib4from - k 1 b c d + + + a b c d
"""

S[48] = """ == text join rev split text "" ""
"""

S[49] = """ if == n 0 % 1 p % * 2 modp - n 1 p p
"""

S[52] = """ if == len l 0 true and < else at l 0 0 t belowthreshold slice l 1 len l t
"""

S[53] = """ + x y
"""

S[55] = """ if < n 2 n + fib - n 1 fib - n 2
"""

S[56] = """ depthok split brackets "" 0
fn depthok cs:list str d:i64 -> bool
 ex depthok ["<" ">"] 0 = true
 if < d 0 false if == len cs 0 == d 0 depthok slice cs 1 len cs + d if == else at cs 0 "" "<" 1 -1
"""

S[57] = """ or == show l show sort l == show l show rev sort l
"""

S[58] = """ keys commonfrom l1 fold @mark dict i64 bool l2 dict i64 bool
fn commonfrom xs:list i64 seen:dict i64 bool acc:dict i64 bool -> dict i64 bool
 ex keys commonfrom [1 2] mark dict i64 bool 2 dict i64 bool = [2]
 let h else at xs 0 0
 if == len xs 0 acc commonfrom slice xs 1 len xs seen if has get seen h put acc h true acc
fn mark seen:dict i64 bool x:i64 -> dict i64 bool
 ex size mark dict i64 bool 1 = 1
 put seen x true
"""

S[59] = """ lpffrom n 2
fn lpffrom m:i64 d:i64 -> i64
 ex lpffrom 15 2 = 5
 if > * d d m m if == % m d 0 lpffrom / m d d lpffrom m + d 1
"""

S[60] = """ sum range 1 + n 1
"""

S[62] = """ derivfrom xs 1 empty i64
fn derivfrom ys:list i64 i:i64 acc:list i64 -> list i64
 ex derivfrom [3 1 2] 1 empty i64 = [1 4]
 if >= i len ys acc derivfrom ys + i 1 push acc * i else at ys i 0
"""

S[64] = """ + len filter @isvowel split s "" if == lower else at split s "" - count s 1 "" "y" 1 0
fn isvowel c:str -> bool
 ex isvowel "a" = true
 contains "aeiouAEIOU" c
"""

S[72] = """ and <= sum q w == show q show rev q
"""

S[73] = """ changesfrom arr 0
fn changesfrom xs:list i64 i:i64 -> i64
 ex changesfrom [1 2] 0 = 1
 if >= i / len xs 2 0 + if != else at xs i 0 else at xs - - len xs 1 i 0 1 0 changesfrom xs + i 1
"""

S[79] = """ cat cat "db" binary decimal "db"
fn binary n:i64 -> str
 ex binary 5 = "101"
 if < n 2 show n cat binary / n 2 show % n 2
"""

S[83] = """ if == n 1 1 * 18 pow10 - n 2
fn pow10 k:i64 -> i64
 ex pow10 2 = 100
 if == k 0 1 * 10 pow10 - k 1
"""

S[85] = """ addfrom lst 1
fn addfrom xs:list i64 i:i64 -> i64
 ex addfrom [4 2 6 7] 1 = 2
 let x else at xs i 0
 if >= i len xs 0 + if == % x 2 0 x 0 addfrom xs + i 2
"""

S[96] = """ filter @prime range 2 n
fn prime m:i64 -> bool
 ex prime 7 = true
 primefrom m 2
fn primefrom m:i64 k:i64 -> bool
 ex primefrom 9 2 = false
 if > * k k m true if == % m k 0 false primefrom m + k 1
"""

S[97] = """ * % magnitude a 10 % magnitude b 10
fn magnitude n:i64 -> i64
 ex magnitude -3 = 3
 if < n 0 neg n n
"""

S[100] = """ pilefrom n 0 empty i64
fn pilefrom m:i64 i:i64 acc:list i64 -> list i64
 ex pilefrom 2 0 empty i64 = [2 4]
 if >= i m acc pilefrom m + i 1 push acc + m * 2 i
"""

S[101] = """ filter @nonempty split join split s "," " " " "
fn nonempty w:str -> bool
 ex nonempty "" = false
 > count w 0
"""

S[102] = """ if > x y -1 if == % y 2 0 y if == x y -1 - y 1
"""

S[106] = """ map @term range 1 + n 1
fn term i:i64 -> i64
 ex term 3 = 6
 if == % i 2 0 fold @times 1 range 1 + i 1 sum range 1 + i 1
fn times a:i64 b:i64 -> i64
 ex times 2 3 = 6
 * a b
"""

S[114] = """ minfrom nums 1 else at nums 0 0 else at nums 0 0
fn minfrom xs:list i64 i:i64 cur:i64 best:i64 -> i64
 ex minfrom [2 -1] 1 2 2 = -1
 let x else at xs i 0
 let c if < cur 0 + cur x x
 if >= i len xs best minfrom xs + i 1 c if < c best c best
"""

S[121] = """ solfrom lst 0
fn solfrom xs:list i64 i:i64 -> i64
 ex solfrom [5 8 7 1] 0 = 12
 let x else at xs i 0
 if >= i len xs 0 + if != % x 2 0 x 0 solfrom xs + i 2
"""

S[122] = """ sum filter @short slice arr 0 k
fn short x:i64 -> bool
 ex short 12 = true
 <= count show x 2
"""

S[123] = """ sort collatzodd n empty i64
fn collatzodd m:i64 acc:list i64 -> list i64
 ex collatzodd 5 empty i64 = [5 1]
 let more if == % m 2 1 push acc m acc
 if == m 1 more collatzodd if == % m 2 0 / m 2 + * 3 m 1 more
"""

S[126] = """ and == show lst show sort lst <= fold @bigger 0 vals fold @bump dict i64 i64 lst 2
fn bigger a:i64 b:i64 -> i64
 ex bigger 1 2 = 2
 if > b a b a
fn bump counts:dict i64 i64 x:i64 -> dict i64 i64
 ex size bump dict i64 i64 1 = 1
 put counts x + else get counts x 0 1
"""

S[131] = """ let ds filter @odd map @digit split show n ""
 if == len ds 0 0 fold @times 1 ds
fn digit c:str -> i64
 ex digit "7" = 7
 else int c 0
fn odd d:i64 -> bool
 ex odd 3 = true
 == % d 2 1
fn times a:i64 b:i64 -> i64
 ex times 2 3 = 6
 * a b
"""

S[139] = """ fold @times 1 map @factorial range 1 + n 1
fn factorial k:i64 -> i64
 ex factorial 3 = 6
 fold @times 1 range 1 + k 1
fn times a:i64 b:i64 -> i64
 ex times 2 3 = 6
 * a b
"""

S[142] = """ sqfrom lst 0
fn sqfrom xs:list i64 i:i64 -> i64
 ex sqfrom [1 2 3] 0 = 6
 let x else at xs i 0
 if >= i len xs 0 + if == % i 3 0 * x x if == % i 4 0 * * x x x x sqfrom xs + i 1
"""

S[150] = """ if and > n 1 nodivisor n 2 x y
fn nodivisor m:i64 k:i64 -> bool
 ex nodivisor 7 2 = true
 if > * k k m true if == % m k 0 false nodivisor m + k 1
"""

S[152] = """ diffsfrom game guess 0 empty i64
fn diffsfrom a:list i64 b:list i64 i:i64 acc:list i64 -> list i64
 ex diffsfrom [1 2] [2 2] 0 empty i64 = [1 0]
 if >= i len a acc diffsfrom a b + i 1 push acc magnitude - else at a i 0 else at b i 0
fn magnitude n:i64 -> i64
 ex magnitude -3 = 3
 if < n 0 neg n n
"""

S[157] = """ or or == + * a a * b b * c c == + * a a * c c * b b == + * b b * c c * a a
"""

S[159] = """ if <= need remaining [+ number need - remaining need] [+ number remaining 0]
"""

S[163] = """ filter @even range bigger 2 smaller a b + smaller 8 bigger a b 1
fn bigger x:i64 y:i64 -> i64
 ex bigger 1 2 = 2
 if > x y x y
fn even x:i64 -> bool
 ex even 2 = true
 == % x 2 0
fn smaller x:i64 y:i64 -> i64
 ex smaller 1 2 = 1
 if < x y x y
"""
