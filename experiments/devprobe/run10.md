### rpn
```aleph
fn rpnop t:str rest:list str st:list i64 -> res list i64 str
 ex rpnop "+" [] [3 4] = ok [7]
 let ob at st - len st 1
 let oa at st - len st 2
 if not has ob err "bad"
 if not has oa err "bad"
 let a else oa 0
 let b else ob 0
 let v if == t "+" + a b if == t "-" - a b * a b
 rpnstep rest push slice st 0 - len st 2 v

fn rpnstep ts:list str st:list i64 -> res list i64 str
 ex rpnstep ["3" "4" "+"] empty i64 = ok [7]
 if == len ts 0 ok st
 let t else at ts 0 ""
 let rest slice ts 1 len ts
 if or == t "+" or == t "-" == t "*" rpnop t rest st
 let p int t
 if not isok p err "bad"
 rpnstep rest push st else p 0

fn rpn s:str -> res i64 str
 ex rpn "3 4 + 2 *" = ok 14
 ex isok rpn "1 +" = false
 let r rpnstep fields s empty i64
 if not isok r err "bad"
 let st else r empty i64
 if != len st 1 err "bad"
 ok else at st 0 0
```

### merge
```aleph
fn merge a:list i64 b:list i64 -> list i64
 ex merge [1 4] [2 3 5] = [1 2 3 4 5]
 ex merge empty i64 empty i64 = empty i64
 if == len a 0 b
 if == len b 0 a
 let x else at a 0 0
 let y else at b 0 0
 if <= x y push merge slice a 1 len a b x
 push merge a slice b 1 len b y
```

### runs
```aleph
fn takeeq xs:list i64 x:i64 -> list i64
 ex takeeq [1 1 2] 1 = [1 1]
 if == len xs 0 empty i64
 if != else at xs 0 0 x empty i64
 push takeeq slice xs 1 len xs x else at xs 0 0

fn dropeq xs:list i64 x:i64 -> list i64
 ex dropeq [1 1 2] 1 = [2]
 if == len xs 0 empty i64
 if != else at xs 0 0 x xs
 dropeq slice xs 1 len xs x

fn runsgo xs:list i64 -> list list i64
 ex runsgo [1 1 2 1] = [[1 1] [2] [1]]
 if == len xs 0 empty list i64
 let x else at xs 0 0
 push runsgo dropeq xs x takeeq xs x

fn runs xs:list i64 -> list list i64
 ex runs [1 1 2 1] = [[1 1] [2] [1]]
 ex runs empty i64 = empty list i64
 runsgo xs
```

### toroman
```aleph
fn trgo n:i64 vs:list i64 ss:list str acc:str -> str
 ex trgo 4 [4 1] ["IV" "I"] "" = "IV"
 if == n 0 acc
 if >= n else at vs 0 0 trgo - n else at vs 0 0 slice vs 1 len vs slice ss 1 len ss cat acc else at ss 0 ""
 trgo n slice vs 1 len vs slice ss 1 len ss acc

fn toroman n:i64 -> str
 ex toroman 1994 = "MCMXCIV"
 ex toroman 3 = "III"
 trgo n [1000 900 500 400 100 90 50 40 10 9 5 4 1] ["M" "CM" "D" "CD" "C" "XC" "L" "XL" "X" "IX" "V" "IV" "I"] ""
```

### fromroman
```aleph
fn rval c:str -> i64
 ex rval "M" = 1000
 ex rval "V" = 5
 if == c "M" 1000
 if == c "D" 500
 if == c "C" 100
 if == c "L" 50
 if == c "X" 10
 if == c "V" 5
 if == c "I" 1
 0

fn frgo cs:list str -> i64
 ex frgo ["I" "V"] = 4
 if == len cs 0 0
 let v rval else at cs 0 ""
 if == len cs 1 v
 let n rval else at cs 1 ""
 if < v n - v frgo slice cs 1 len cs
 + v frgo slice cs 1 len cs

fn fromroman s:str -> i64
 ex fromroman "MCMXCIV" = 1994
 ex fromroman "III" = 3
 frgo split s ""
```

### evalexpr
```aleph
type expr | num v:i64 | add a:expr b:expr | mul a:expr b:expr
fn evalexpr e:expr -> i64
 ex evalexpr add num 2 mul num 3 num 4 = 14
 ex evalexpr num 5 = 5
 match e num v v add a b + evalexpr a evalexpr b mul a b * evalexpr a evalexpr b
```

### topn
```aleph
fn topn s:str k:i64 -> list str
 ex topn "b a b c a b" 2 = ["b" "a"]
 ex topn "" 3 = empty str
 let cnt fold lam ac wo put ac wo + else get ac wo 0 1 dict str i64 fields s
 let w0 sort keys cnt
 let w1 sortby lam wd neg else get cnt wd 0 w0
 slice w1 0 k
```

### transpose
```aleph
fn trcell m:list list i64 col:i64 -> list i64
 ex trcell [[1 2 3] [4 5 6]] 1 = [2 5]
 map lam row else at row col 0 m

fn transpose m:list list i64 -> list list i64
 ex transpose [[1 2 3] [4 5 6]] = [[1 4] [2 5] [3 6]]
 ex transpose empty list i64 = empty list i64
 if == len m 0 empty list i64
 let c0 else at m 0 empty i64
 map lam ci trcell m ci range 0 len c0
```

### brackets
```aleph
fn bropen c:str -> bool
 ex bropen "(" = true
 or == c "(" or == c "[" == c "{"

fn brclose c:str -> bool
 ex brclose "]" = true
 or == c ")" or == c "]" == c "}"

fn brmatch o:str c:str -> bool
 ex brmatch "(" ")" = true
 or and == o "(" == c ")" or and == o "[" == c "]" and == o "{" == c "}"

fn brgo cs:list str st:list str -> bool
 ex brgo ["(" ")"] empty str = true
 if == len cs 0 == len st 0
 let c else at cs 0 ""
 let rest slice cs 1 len cs
 if bropen c brgo rest push st c
 if brclose c if == len st 0 false if brmatch else at st - len st 1 "" c brgo rest slice st 0 - len st 1 false
 brgo rest st

fn brackets s:str -> bool
 ex brackets "([]{x})" = true
 ex brackets "(]" = false
 brgo split s "" empty str
```

### wrap
```aleph
fn wrapgo ws:list str w:i64 cur:str acc:list str -> list str
 ex wrapgo ["a" "bb" "ccc"] 4 "" empty str = ["a bb" "ccc"]
 if == len ws 0 if == count cur 0 rev acc rev push acc cur
 let word else at ws 0 ""
 let rest slice ws 1 len ws
 if == count cur 0 wrapgo rest w word acc
 let cand cat cat cur " " word
 if <= count cand w wrapgo rest w cand acc
 wrapgo rest w word push acc cur

fn wrap s:str w:i64 -> list str
 ex wrap "a bb ccc d" 4 = ["a bb" "ccc" "d"]
 ex wrap "" 3 = empty str
 wrapgo fields s w "" empty str
```

### bank
```aleph
fn bankgo cs:list str bal:i64 -> res i64 str
 ex bankgo ["dep 10" "wd 3"] 0 = ok 7
 if == len cs 0 ok bal
 let cmd else at cs 0 ""
 let rest slice cs 1 len cs
 let ps split cmd " "
 if != len ps 2 err "bad"
 let op else at ps 0 ""
 let ns else at ps 1 ""
 if not or == op "dep" == op "wd" err "bad"
 let n int ns
 if not isok n err "bad"
 let v else n 0
 if < v 0 err "bad"
 if == op "dep" bankgo rest + bal v
 if < bal v err "overdraft"
 bankgo rest - bal v

fn bank cs:list str -> res i64 str
 ex bank ["dep 10" "wd 3"] = ok 7
 ex bank ["wd 1"] = err "overdraft"
 bankgo cs 0
```

### jsonsum
```aleph
fn jssum xs:list json -> res i64 str
 ex jssum [json 1 json 2] = ok 3
 let ns try mapr lam v okor jint v "bad" xs
 ok sum ns

fn jsonsum s:str -> res i64 str
 ex jsonsum "[1, 2, 3]" = ok 6
 ex isok jsonsum "[1, \"a\"]" = false
 let jp jparse s
 if not isok jp err "bad"
 let j else jp json 0
 let xs okor jlist j "bad"
 let n try jssum xs
 ok n
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
 some / + tof else at ys - / n 2 1 0 tof else at ys / n 2 0 2.0
```

### primes
```aleph
fn hasdiv p:i64 d:i64 -> bool
 ex hasdiv 9 2 = true
 if > * d d p false
 if == % p d 0 true
 hasdiv p + d 1

fn isprime p:i64 -> bool
 ex isprime 7 = true
 ex isprime 9 = false
 if < p 2 false
 not hasdiv p 2

fn primes n:i64 -> list i64
 ex primes 10 = [2 3 5 7]
 ex primes 1 = empty i64
 if < n 2 empty i64
 filter @isprime range 2 + n 1
```

### titlecase
```aleph
fn cap w:str -> str
 ex cap "wORLD" = "World"
 ex cap "" = ""
 if == count w 0 ""
 let cs split w ""
 cat upper else at cs 0 "" lower join slice cs 1 len cs ""

fn titlecase s:str -> str
 ex titlecase "hello  wORLD" = "Hello World"
 ex titlecase "" = ""
 join map @cap fields s " "
```

### gcdlist
```aleph
fn gcd a:i64 b:i64 -> i64
 ex gcd 12 18 = 6
 if == b 0 a
 gcd b % a b

fn gcdlist xs:list i64 -> i64
 ex gcdlist [12 18 30] = 6
 ex gcdlist empty i64 = 0
 fold lam acc x gcd acc x 0 xs
```

### kvdict
```aleph
fn kvgo ps:list str d:dict str i64 -> res dict str i64 str
 ex kvgo ["a=1" "b=2"] dict str i64 = ok put put dict str i64 "a" 1 "b" 2
 if == len ps 0 ok d
 let p else at ps 0 ""
 let kv split p "="
 if != len kv 2 err "bad"
 let key else at kv 0 ""
 if == count key 0 err "bad"
 let vs else at kv 1 ""
 let n int vs
 if not isok n err "bad"
 kvgo slice ps 1 len ps put d key else n 0

fn kvdict s:str -> res dict str i64 str
 ex kvdict "a=1,b=2" = ok put put dict str i64 "a" 1 "b" 2
 ex isok kvdict "a" = false
 if == count s 0 ok dict str i64
 kvgo split s "," dict str i64
```

### leaves
```aleph
type tree | leaf v:i64 | node l:tree r:tree
fn leaves t:tree -> list i64
 ex leaves node leaf 1 node leaf 2 leaf 3 = [1 2 3]
 ex leaves leaf 4 = [4]
 match t leaf v [v] node l r cat leaves l leaves r
```

### reachable
```aleph
fn reachgo g:dict str list str todo:list str seen:dict str bool -> dict str bool
 ex size reachgo dict str list str ["x"] dict str bool = 1
 if == len todo 0 seen
 let n else at todo 0 ""
 let rest slice todo 1 len todo
 if else get seen n false reachgo g rest seen
 let nbs else get g n empty str
 reachgo g cat rest nbs put seen n true

fn reachable g:dict str list str s:str -> i64
 ex reachable put put dict str list str "a" ["b"] "b" ["a" "c"] "a" = 3
 ex reachable dict str list str "x" = 1
 size reachgo g [s] put dict str bool s true
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
