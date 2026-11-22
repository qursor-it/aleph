### rpn
```aleph
fn binop op:str st:list i64 -> res list i64 str
 ex binop "+" [1 2] = ok [3]
 let n len st
 if < n 2 err "bad"
 let b else at st - n 1 0
 let a else at st - n 2 0
 let v if == op "+" + a b if == op "-" - a b * a b
 ok push slice st 0 - n 2 v
fn rpnloop toks:list str st:list i64 -> res list i64 str
 ex isok rpnloop ["1" "2" "+"] empty i64 = true
 if == len toks 0 ok st
 let t else at toks 0 ""
 let rest slice toks 1 len toks
 if or == t "+" or == t "-" == t "*" rpnloop rest try binop t st
 if isok int t rpnloop rest push st else int t 0
 err "bad"
fn rpn s:str -> res i64 str
 ex rpn "3 4 + 2 *" = ok 14
 ex isok rpn "1 +" = false
 let st try rpnloop fields s empty i64
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
fn runsgo rest:list i64 cur:list i64 acc:list list i64 -> list list i64
 ex runsgo [2 2 1] [1] empty list i64 = [[1] [2 2] [1]]
 if == len rest 0 push acc cur
 let h else at rest 0 0
 let last else at cur - len cur 1 0
 if == h last runsgo slice rest 1 len rest push cur h acc
 runsgo slice rest 1 len rest [h] push acc cur
fn runs xs:list i64 -> list list i64
 ex runs [1 1 2 1] = [[1 1] [2] [1]]
 ex runs empty i64 = empty list i64
 if == len xs 0 empty list i64
 runsgo slice xs 1 len xs [else at xs 0 0] empty list i64
```

### toroman
```aleph
fn toromant n:i64 vals:list i64 syms:list str -> str
 ex toromant 4 [5 4 1] ["V" "IV" "I"] = "IV"
 if == n 0 ""
 let v else at vals 0 0
 let sym else at syms 0 ""
 if >= n v cat sym toromant - n v vals syms
 toromant n slice vals 1 len vals slice syms 1 len syms
fn toroman n:i64 -> str
 ex toroman 1994 = "MCMXCIV"
 ex toroman 3 = "III"
 toromant n [1000 900 500 400 100 90 50 40 10 9 5 4 1] ["M" "CM" "D" "CD" "C" "XC" "L" "XL" "X" "IX" "V" "IV" "I"]
```

### fromroman
```aleph
fn rval c:str -> i64
 ex rval "M" = 1000
 if == c "I" 1 if == c "V" 5 if == c "X" 10 if == c "L" 50 if == c "C" 100 if == c "D" 500 if == c "M" 1000 0
fn fromr vs:list i64 -> i64
 ex fromr [1 10] = 9
 if == len vs 0 0
 let h else at vs 0 0
 if == len vs 1 h
 let n else at vs 1 0
 if < h n + neg h fromr slice vs 1 len vs
 + h fromr slice vs 1 len vs
fn fromroman s:str -> i64
 ex fromroman "MCMXCIV" = 1994
 ex fromroman "III" = 3
 fromr map @rval split s ""
```

### evalexpr
```aleph
type expr | num v:i64 | add a:expr b:expr | mul a:expr b:expr
fn evalexpr e:expr -> i64
 ex evalexpr add num 2 mul num 3 num 4 = 14
 ex evalexpr num 5 = 5
 match e num nv nv add xa xb + evalexpr xa evalexpr xb mul ya yb * evalexpr ya evalexpr yb
```

### topn
```aleph
fn topn s:str k:i64 -> list str
 ex topn "b a b c a b" 2 = ["b" "a"]
 ex topn "" 3 = empty str
 let ws fields s
 let d fold lam acc wd put acc wd + else get acc wd 0 1 dict str i64 ws
 let u uniq sort ws
 let byfreq sortby lam wd2 neg else get d wd2 0 u
 slice byfreq 0 k
```

### transpose
```aleph
fn tcol m:list list i64 i:i64 -> list i64
 ex tcol [[1 2] [3 4]] 1 = [2 4]
 map lam row else at row i 0 m
fn transpose m:list list i64 -> list list i64
 ex transpose [[1 2 3] [4 5 6]] = [[1 4] [2 5] [3 6]]
 ex transpose empty list i64 = empty list i64
 if == len m 0 empty list i64
 let w len else at m 0 empty i64
 map lam i tcol m i range 0 w
```

### brackets
```aleph
fn bclose st:list str c:str -> res list str str
 ex isok bclose ["("] ")" = true
 if == len st 0 err "bad"
 let t else at st - len st 1 ""
 let m if == c ")" "(" if == c "]" "[" "{"
 if != t m err "bad"
 ok slice st 0 - len st 1
fn bpush st:list str c:str -> res list str str
 ex isok bpush empty str "(" = true
 if or == c "(" or == c "[" == c "{" ok push st c
 if or == c ")" or == c "]" == c "}" bclose st c
 ok st
fn bloop cs:list str st:list str -> res list str str
 ex isok bloop split "()" "" empty str = true
 if == len cs 0 ok st
 let c else at cs 0 ""
 let st2 try bpush st c
 bloop slice cs 1 len cs st2
fn brackets s:str -> bool
 ex brackets "([]{x})" = true
 ex brackets "(]" = false
 let cs split s ""
 let r bloop cs empty str
 and isok r == len else r empty str 0
```

### wrap
```aleph
fn wraploop ws:list str cur:str acc:list str w:i64 -> list str
 ex wraploop ["a" "bb"] "" empty str 4 = ["a bb"]
 if and == len ws 0 == cur "" acc
 if == len ws 0 push acc cur
 let h else at ws 0 ""
 let rest slice ws 1 len ws
 if == cur "" wraploop rest h acc w
 if <= + count cur + 1 count h w wraploop rest cat cat cur " " h acc w
 wraploop rest h push acc cur w
fn wrap s:str w:i64 -> list str
 ex wrap "a bb ccc d" 4 = ["a bb" "ccc" "d"]
 ex wrap "" 3 = empty str
 wraploop fields s "" empty str w
```

### bank
```aleph
fn bankstep bal:i64 c:str -> res i64 str
 ex bankstep 10 "dep 5" = ok 15
 ex bankstep 3 "wd 5" = err "overdraft"
 let parts fields c
 if != len parts 2 err "bad"
 let op else at parts 0 ""
 let a else at parts 1 ""
 if not isok int a err "bad"
 let n else int a 0
 if < n 0 err "bad"
 if == op "dep" ok + bal n
 if and == op "wd" < bal n err "overdraft"
 if == op "wd" ok - bal n
 err "bad"
fn bankloop cs:list str bal:i64 -> res i64 str
 ex bankloop ["dep 10" "wd 3"] 0 = ok 7
 if == len cs 0 ok bal
 let c else at cs 0 ""
 let b try bankstep bal c
 bankloop slice cs 1 len cs b
fn bank cs:list str -> res i64 str
 ex bank ["dep 10" "wd 3"] = ok 7
 ex bank ["wd 1"] = err "overdraft"
 bankloop cs 0
```

### jsonsum
```aleph
fn jsum xs:list json -> res i64 str
 ex jsum [json 1 json 2] = ok 3
 if == len xs 0 ok 0
 let h else at xs 0 json 0
 if not has jint h err "bad"
 let n else jint h 0
 let rest try jsum slice xs 1 len xs
 ok + n rest
fn jsonsum s:str -> res i64 str
 ex jsonsum "[1, 2, 3]" = ok 6
 ex isok jsonsum "[1, \"a\"]" = false
 let j try okor jparse s "bad"
 if not has jlist j err "bad"
 let xs else jlist j empty json
 jsum xs
```

### median
```aleph
fn median xs:list i64 -> opt f64
 ex median [3 1 2] = some 2.0
 ex median empty i64 = none f64
 let ys sort xs
 let n len ys
 if == n 0 none f64
 if == % n 2 1 some tof else at ys / n 2 0
 / + tof else at ys - / n 2 1 0 tof else at ys / n 2 0 2.0
```

### primes
```aleph
fn primep p:i64 -> bool
 ex primep 7 = true
 ex primep 1 = false
 if < p 2 false
 == len filter lam d == % p d 0 range 2 p 0
fn primes n:i64 -> list i64
 ex primes 10 = [2 3 5 7]
 ex primes 1 = empty i64
 if < n 2 empty i64
 filter @primep range 2 + n 1
```

### titlecase
```aleph
fn titleword w:str -> str
 ex titleword "wORLD" = "World"
 let cs split w ""
 let f else at cs 0 ""
 let r join slice cs 1 len cs ""
 cat upper f lower r
fn titlecase s:str -> str
 ex titlecase "hello  wORLD" = "Hello World"
 ex titlecase "" = ""
 join map @titleword fields s " "
```

### gcdlist
```aleph
fn gcd2 a:i64 b:i64 -> i64
 ex gcd2 12 18 = 6
 if == b 0 a
 gcd2 b % a b
fn gcdlist xs:list i64 -> i64
 ex gcdlist [12 18 30] = 6
 ex gcdlist empty i64 = 0
 fold @gcd2 0 xs
```

### kvdict
```aleph
fn kvpair d:dict str i64 p:str -> res dict str i64 str
 ex kvpair dict str i64 "a=1" = ok put dict str i64 "a" 1
 let parts split p "="
 if != len parts 2 err "bad"
 let k else at parts 0 ""
 let v else at parts 1 ""
 if or == k "" not isok int v err "bad"
 ok put d k else int v 0
fn kvloop d:dict str i64 ps:list str -> res dict str i64 str
 ex kvloop dict str i64 ["a=1"] = ok put dict str i64 "a" 1
 if == len ps 0 ok d
 let p else at ps 0 ""
 let d2 try kvpair d p
 kvloop d2 slice ps 1 len ps
fn kvdict s:str -> res dict str i64 str
 ex kvdict "a=1,b=2" = ok put put dict str i64 "a" 1 "b" 2
 ex isok kvdict "a" = false
 if == s "" ok dict str i64
 kvloop dict str i64 split s ","
```

### leaves
```aleph
type tree | leaf v:i64 | node l:tree r:tree
fn leaves t:tree -> list i64
 ex leaves node leaf 1 node leaf 2 leaf 3 = [1 2 3]
 ex leaves leaf 4 = [4]
 match t leaf lv [lv] node nl nr cat leaves nl leaves nr
```

### reachable
```aleph
fn reachgo g:dict str list str frontier:list str seen:dict str bool -> dict str bool
 ex size reachgo dict str list str ["a"] put dict str bool "a" true = 1
 if == len frontier 0 seen
 let x else at frontier 0 ""
 let rest slice frontier 1 len frontier
 let seen2 if has get seen x seen put seen x true
 let ns else get g x empty str
 let newn filter lam nb not has get seen2 nb ns
 reachgo g cat rest newn seen2
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
 let u uniq sort xs
 if < len u 2 none i64
 at u - len u 2
```
