### rpn

```aleph
fn rpnfinal st:list i64 -> res i64 str
 ex rpnfinal [5] = ok 5
 ex isok rpnfinal [1 2] = false
 if == len st 1 ok else at st 0 0
 err "bad"
fn rpncalc st:list i64 op:str -> res list i64 str
 ex rpncalc [2 3] "+" = ok [5]
 ex isok rpncalc [2] "+" = false
 if < len st 2 err "bad"
 let idx - len st 2
 let a else at st idx 0
 let b else at st + idx 1 0
 let v if == op "+" + a b if == op "-" - a b * a b
 ok push slice st 0 idx v
fn rpnapply st:list i64 t:str -> res list i64 str
 ex rpnapply empty i64 "7" = ok [7]
 ex isok rpnapply empty i64 "+" = false
 if == t "+" rpncalc st "+"
 if == t "-" rpncalc st "-"
 if == t "*" rpncalc st "*"
 if not isok int t err "bad"
 let n else int t 0
 ok push st n
fn rpneval ts:list str st:list i64 -> res i64 str
 ex rpneval ["1" "2" "+"] empty i64 = ok 3
 ex isok rpneval ["1" "+"] empty i64 = false
 if == len ts 0 rpnfinal st
 let t else at ts 0 ""
 let r try rpnapply st t
 rpneval slice ts 1 len ts r
fn rpn s:str -> res i64 str
 ex rpn "3 4 + 2 *" = ok 14
 ex isok rpn "1 +" = false
 rpneval split s " " empty i64
```

### merge

```aleph
fn mergego a:list i64 b:list i64 i:i64 j:i64 -> list i64
 ex mergego [1 3] [2] 0 0 = [1 2 3]
 ex mergego empty i64 empty i64 0 0 = empty i64
 if == i len a slice b j len b
 if == j len b slice a i len a
 let x else at a i 0
 let y else at b j 0
 if <= x y push mergego a b + i 1 j x push mergego a b i + j 1 y
fn merge a:list i64 b:list i64 -> list i64
 ex merge [1 4] [2 3 5] = [1 2 3 4 5]
 ex merge empty i64 empty i64 = empty i64
 mergego a b 0 0
```

### runs

```aleph
fn runsfin acc:list list i64 cur:list i64 -> list list i64
 ex runsfin empty list i64 empty i64 = empty list i64
 ex runsfin empty list i64 [1] = [[1]]
 if == len cur 0 acc
 push acc cur
fn runsgo xs:list i64 i:i64 cur:list i64 acc:list list i64 -> list list i64
 ex runsgo [1 1] 0 empty i64 empty list i64 = [[1 1]]
 ex runsgo [1 1 2] 0 empty i64 empty list i64 = [[1 1] [2]]
 if == i len xs runsfin acc cur
 let x else at xs i 0
 if == len cur 0 runsgo xs + i 1 push cur x acc
 let last else at cur - len cur 1 0
 if == last x runsgo xs + i 1 push cur x acc
 runsgo xs + i 1 [x] push acc cur
fn runs xs:list i64 -> list list i64
 ex runs [1 1 2 1] = [[1 1] [2] [1]]
 ex runs empty i64 = empty list i64
 runsgo xs 0 empty i64 empty list i64
```

### toroman

```aleph
fn toromanh n:i64 -> str
 ex toromanh 1994 = "MCMXCIV"
 ex toromanh 4 = "IV"
 if >= n 1000 cat "M" toromanh - n 1000
 if >= n 900 cat "CM" toromanh - n 900
 if >= n 500 cat "D" toromanh - n 500
 if >= n 400 cat "CD" toromanh - n 400
 if >= n 100 cat "C" toromanh - n 100
 if >= n 90 cat "XC" toromanh - n 90
 if >= n 50 cat "L" toromanh - n 50
 if >= n 40 cat "XL" toromanh - n 40
 if >= n 10 cat "X" toromanh - n 10
 if >= n 9 cat "IX" toromanh - n 9
 if >= n 5 cat "V" toromanh - n 5
 if >= n 4 cat "IV" toromanh - n 4
 if >= n 1 cat "I" toromanh - n 1
 ""
fn toroman n:i64 -> str
 ex toroman 1994 = "MCMXCIV"
 ex toroman 3 = "III"
 toromanh n
```

### fromroman

```aleph
fn romanval c:str -> i64
 ex romanval "M" = 1000
 ex romanval "I" = 1
 if == c "I" 1
 if == c "V" 5
 if == c "X" 10
 if == c "L" 50
 if == c "C" 100
 if == c "D" 500
 if == c "M" 1000
 0
fn fromromango cs:list str i:i64 prev:i64 total:i64 -> i64
 ex fromromango split "IV" "" 0 0 0 = 4
 ex fromromango split "III" "" 0 0 0 = 3
 if == i len cs total
 let v romanval else at cs i ""
 if > v prev fromromango cs + i 1 v - + total v * 2 prev
 fromromango cs + i 1 v + total v
fn fromroman s:str -> i64
 ex fromroman "MCMXCIV" = 1994
 ex fromroman "III" = 3
 fromromango split s "" 0 0 0
```

### evalexpr

```aleph
type expr | num v:i64 | add a:expr b:expr | mul a:expr b:expr
fn evalexpr e:expr -> i64
 ex evalexpr add num 2 mul num 3 num 4 = 14
 ex evalexpr num 5 = 5
 match e num v v add x y + evalexpr x evalexpr y mul x y * evalexpr x evalexpr y
```

### topn

```aleph
fn topcounts s:str -> dict str i64
 ex get topcounts "a a b" "a" = some 2
 fold lam d w put d w + else get d w 0 1 dict str i64 fields s
fn topn s:str k:i64 -> list str
 ex topn "b a b c a b" 2 = ["b" "a"]
 ex topn "" 3 = empty str
 if == len fields s 0 empty str
 let d topcounts s
 let ws sort keys d
 slice sortby lam w neg else get d w 0 ws 0 k
```

### transpose

```aleph
fn col m:list list i64 j:i64 i:i64 -> list i64
 ex col [[1 2] [3 4]] 0 0 = [1 3]
 ex col [[1 2] [3 4]] 1 0 = [2 4]
 if == i len m empty i64
 push col m j + i 1 else at else at m i empty i64 j 0
fn transposego m:list list i64 j:i64 cols:i64 -> list list i64
 ex transposego [[1 2] [3 4]] 0 2 = [[1 3] [2 4]]
 if == j cols empty list i64
 push transposego m + j 1 cols col m j 0
fn transpose m:list list i64 -> list list i64
 ex transpose [[1 2 3] [4 5 6]] = [[1 4] [2 5] [3 6]]
 ex transpose empty list i64 = empty list i64
 if == len m 0 empty list i64
 transposego m 0 len else at m 0 empty i64
```

### brackets

```aleph
fn bracketgo cs:list str st:list str -> bool
 ex bracketgo split "([])" "" empty str = true
 ex bracketgo split "(]" "" empty str = false
 if == len cs 0 == len st 0
 let c else at cs 0 ""
 let rest slice cs 1 len cs
 if == c "(" bracketgo rest push st c
 if == c "[" bracketgo rest push st c
 if == c "{" bracketgo rest push st c
 let close or == c ")" or == c "]" == c "}"
 let top else at st - len st 1 ""
 let okc if == c ")" == top "(" if == c "]" == top "[" == top "{"
 if close if not okc false bracketgo rest slice st 0 - len st 1
 bracketgo rest st
fn brackets s:str -> bool
 ex brackets "([]{x})" = true
 ex brackets "(]" = false
 bracketgo split s "" empty str
```

### wrap

```aleph
fn wrapfin acc:list str cur:str -> list str
 ex wrapfin ["a"] "" = ["a"]
 ex wrapfin ["a"] "b" = ["a" "b"]
 if == cur "" acc
 push acc cur
fn wrapgo ws:list str w:i64 i:i64 cur:str acc:list str -> list str
 ex wrapgo ["a" "bb"] 4 0 "" empty str = ["a bb"]
 if == i len ws wrapfin acc cur
 let word else at ws i ""
 let rest slice ws 1 len ws
 if == cur "" wrapgo ws w + i 1 word acc
 if <= + count cur + 1 count word w wrapgo ws w + i 1 cat cat cur " " word acc
 wrapgo ws w + i 1 word push acc cur
fn wrap s:str w:i64 -> list str
 ex wrap "a bb ccc d" 4 = ["a bb" "ccc" "d"]
 ex wrap "" 3 = empty str
 wrapgo fields s w 0 "" empty str
```

### bank

```aleph
fn banksum cs:list str bal:i64 -> res i64 str
 ex banksum ["dep 5"] 0 = ok 5
 if == len cs 0 ok bal
 let c else at cs 0 ""
 let toks fields c
 let rest slice cs 1 len cs
 if != len toks 2 err "bad"
 let cmd else at toks 0 ""
 let arg else at toks 1 ""
 if not isok int arg err "bad"
 let n else int arg 0
 if < n 0 err "bad"
 if == cmd "dep" banksum rest + bal n
 if == cmd "wd" if > n bal err "overdraft" banksum rest - bal n
 err "bad"
fn bank cs:list str -> res i64 str
 ex bank ["dep 10" "wd 3"] = ok 7
 ex bank ["wd 1"] = err "overdraft"
 banksum cs 0
```

### jsonsum

```aleph
fn jsonsum s:str -> res i64 str
 ex jsonsum "[1, 2, 3]" = ok 6
 ex isok jsonsum "[1, \"a\"]" = false
 if not isok jparse s err "bad"
 let j else jparse s json 0
 if not has jlist j err "bad"
 let xs else jlist j empty json
 let ns try mapr lam x if has jint x ok else jint x 0 err "bad" xs
 ok sum ns
```

### median

```aleph
fn median xs:list i64 -> opt f64
 ex median [3 1 2] = some 2.0
 ex median empty i64 = none f64
 if == len xs 0 none f64
 let ys sort xs
 let n len ys
 if == % n 2 1 some tof else at ys / n 2 0
 let a else at ys - / n 2 1 0
 let b else at ys / n 2 0
 some / tof + a b 2.0
```

### primes

```aleph
fn anydiv x:i64 d:i64 -> bool
 ex anydiv 9 2 = true
 ex anydiv 7 2 = false
 if > * d d x false
 if == % x d 0 true
 anydiv x + d 1
fn isprime x:i64 -> bool
 ex isprime 2 = true
 ex isprime 4 = false
 if < x 2 false
 not anydiv x 2
fn primes n:i64 -> list i64
 ex primes 10 = [2 3 5 7]
 ex primes 1 = empty i64
 filter lam p isprime p range 2 + n 1
```

### titlecase

```aleph
fn titleword w:str -> str
 ex titleword "wORLD" = "World"
 ex titleword "" = ""
 if == w "" ""
 let cs split w ""
 let f else at cs 0 ""
 cat upper f join map lam c lower c slice cs 1 len cs ""
fn titlecase s:str -> str
 ex titlecase "hello  wORLD" = "Hello World"
 ex titlecase "" = ""
 join map lam w titleword w fields s " "
```

### gcdlist

```aleph
fn gcd a:i64 b:i64 -> i64
 ex gcd 12 18 = 6
 ex gcd 0 5 = 5
 if == b 0 a
 gcd b % a b
fn gcdlist xs:list i64 -> i64
 ex gcdlist [12 18 30] = 6
 ex gcdlist empty i64 = 0
 fold lam a b gcd a b 0 xs
```

### kvdict

```aleph
fn kvgo ps:list str d:dict str i64 -> res dict str i64 str
 ex kvgo split "a=1" "," dict str i64 = ok put dict str i64 "a" 1
 if == len ps 0 ok d
 let p else at ps 0 ""
 let parts split p "="
 let rest slice ps 1 len ps
 if != len parts 2 err "bad"
 let k else at parts 0 ""
 if == k "" err "bad"
 let v else at parts 1 ""
 if not isok int v err "bad"
 let n else int v 0
 kvgo rest put d k n
fn kvdict s:str -> res dict str i64 str
 ex kvdict "a=1,b=2" = ok put put dict str i64 "a" 1 "b" 2
 ex isok kvdict "a" = false
 if == s "" ok dict str i64
 kvgo split s "," dict str i64
```

### leaves

```aleph
type tree | leaf v:i64 | node l:tree r:tree
fn leaves t:tree -> list i64
 ex leaves node leaf 1 node leaf 2 leaf 3 = [1 2 3]
 ex leaves leaf 4 = [4]
 match t leaf v [v] node x y cat leaves x leaves y
```

### reachable

```aleph
type walk seen:dict str bool todo:list str
fn reach g:dict str list str w:walk -> i64
 ex reach dict str list str walk dict str bool empty str = 0
 if == len .todo w 0 size .seen w
 let n else at .todo w 0 ""
 let rest slice .todo w 1 len .todo w
 let w2 fold lam a nb if has get .seen a nb a walk .seen a push .todo a nb walk .seen w rest else get g n empty str
 reach g w2
fn reachable g:dict str list str s:str -> i64
 ex reachable put put dict str list str "a" ["b"] "b" ["a" "c"] "a" = 3
 ex reachable dict str list str "x" = 1
 reach g walk put dict str bool s true push empty str s
```

### secondmax

```aleph
fn secondmax xs:list i64 -> opt i64
 ex secondmax [3 1 3 2] = some 2
 ex secondmax [5 5] = none i64
 let ys uniq sort xs
 if < len ys 2 none i64
 at ys - len ys 2
```
