### rpn
```aleph
fn rstart -> res list i64 str
 ex isok rstart = true
 ok empty i64
fn applyop st:list i64 op:str -> res list i64 str
 ex isok applyop [1] "+" = false
 ex applyop [3 4] "*" = ok [12]
 let n len st
 if < n 2 err "bad"
 let a else at st - n 2 0
 let b else at st - n 1 0
 let k slice st 0 - n 2
 if == op "+" ok push k + a b
 if == op "-" ok push k - a b
 if == op "*" ok push k * a b
 err "bad"
fn step acc:res list i64 str t:str -> res list i64 str
 ex isok step rstart "+" = false
 ex step rstart "7" = ok [7]
 let st try acc
 if or or == t "+" == t "-" == t "*" applyop st t
 let v okor int t "bad"
 ok push st v
fn rpn s:str -> res i64 str
 ex rpn "3 4 + 2 *" = ok 14
 ex isok rpn "1 +" = false
 let st try fold @step rstart fields s
 if == len st 1 ok else at st 0 0
 err "bad"
```

### merge
```aleph
fn merge a:list i64 b:list i64 -> list i64
 ex merge [1 4] [2 3 5] = [1 2 3 4 5]
 ex merge empty i64 empty i64 = empty i64
 if == len a 0 b
 if == len b 0 a
 if <= else at a 0 0 else at b 0 0 cat [else at a 0 0] merge slice a 1 len a b
 cat [else at b 0 0] merge a slice b 1 len b
```

### runs
```aleph
fn runsgo xs:list i64 cur:list i64 -> list list i64
 ex runsgo [1 2] [1] = [[1] [2]]
 if == len xs 0 [cur]
 let x else at xs 0 0
 let nxt slice xs 1 len xs
 if == x else at cur 0 0 runsgo nxt push cur x
 cat [cur] runsgo nxt [x]
fn runs xs:list i64 -> list list i64
 ex runs [1 1 2 1] = [[1 1] [2] [1]]
 ex runs empty i64 = empty list i64
 if == len xs 0 empty list i64
 runsgo slice xs 1 len xs [else at xs 0 0]
```

### toroman
```aleph
fn toroman n:i64 -> str
 ex toroman 1994 = "MCMXCIV"
 ex toroman 3 = "III"
 if == n 0 ""
 if >= n 1000 cat "M" toroman - n 1000
 if >= n 900 cat "CM" toroman - n 900
 if >= n 500 cat "D" toroman - n 500
 if >= n 400 cat "CD" toroman - n 400
 if >= n 100 cat "C" toroman - n 100
 if >= n 90 cat "XC" toroman - n 90
 if >= n 50 cat "L" toroman - n 50
 if >= n 40 cat "XL" toroman - n 40
 if >= n 10 cat "X" toroman - n 10
 if >= n 9 cat "IX" toroman - n 9
 if >= n 5 cat "V" toroman - n 5
 if >= n 4 cat "IV" toroman - n 4
 cat "I" toroman - n 1
```

### fromroman
```aleph
fn val c:str -> i64
 ex val "X" = 10
 if == c "I" 1
 if == c "V" 5
 if == c "X" 10
 if == c "L" 50
 if == c "C" 100
 if == c "D" 500
 if == c "M" 1000
 0
fn fromroman s:str -> i64
 ex fromroman "MCMXCIV" = 1994
 ex fromroman "III" = 3
 if == len s 0 0
 let v val slice s 0 1
 let rest slice s 1 len s
 if == len rest 0 v
 if < v val slice rest 0 1 - fromroman rest v
 + v fromroman rest
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
 let counts fold lam d w put d w + else get d w 0 1 dict str i64 fields s
 let ws sort keys counts
 slice sortby lam w neg else get counts w 0 ws 0 k
```

### transpose
```aleph
fn transpose m:list list i64 -> list list i64
 ex transpose [[1 2 3] [4 5 6]] = [[1 4] [2 5] [3 6]]
 ex transpose empty list i64 = empty list i64
 if == len m 0 empty list i64
 let cols len else at m 0 empty i64
 map lam j map lam r else at r j 0 m range 0 cols
```

### brackets
```aleph
fn bcheck cs:list str st:list str -> bool
 ex bcheck ["(" ")"] empty str = true
 ex bcheck [")"] empty str = false
 if == len cs 0 == len st 0
 let c else at cs 0 ""
 let rest slice cs 1 len cs
 if not contains "()[]{}" c bcheck rest st
 if or or == c "(" == c "[" == c "{" bcheck rest push st c
 if == len st 0 false
 let top else at st - len st 1 ""
 if or and == c ")" == top "(" or and == c "]" == top "[" and == c "}" == top "{" bcheck rest slice st 0 - len st 1
 false
fn brackets s:str -> bool
 ex brackets "([]{x})" = true
 ex brackets "(]" = false
 bcheck split s "" empty str
```

### wrap
```aleph
fn wrapgo ws:list str cur:str w:i64 -> list str
 ex wrapgo ["bb" "ccc" "d"] "a" 4 = ["a bb" "ccc" "d"]
 ex wrapgo empty str "a" 4 = ["a"]
 if == len ws 0 [cur]
 let x else at ws 0 ""
 let nxt slice ws 1 len ws
 if <= + + len cur 1 len x w wrapgo nxt cat cat cur " " x w
 cat [cur] wrapgo nxt x w
fn wrap s:str w:i64 -> list str
 ex wrap "a bb ccc d" 4 = ["a bb" "ccc" "d"]
 ex wrap "" 3 = empty str
 let ws fields s
 if == len ws 0 empty str
 wrapgo slice ws 1 len ws else at ws 0 "" w
```

### bank
```aleph
fn bzero -> res i64 str
 ex isok bzero = true
 ok 0
fn bankstep acc:res i64 str c:str -> res i64 str
 ex bankstep ok 10 "wd 3" = ok 7
 ex isok bankstep ok 2 "wd 3" = false
 let b try acc
 let ps fields c
 if != len ps 2 err "bad"
 let op else at ps 0 ""
 let n try okor int else at ps 1 "" "bad"
 if < n 0 err "bad"
 if == op "dep" ok + b n
 if and == op "wd" < b n err "overdraft"
 if == op "wd" ok - b n
 err "bad"
fn bank cs:list str -> res i64 str
 ex bank ["dep 10" "wd 3"] = ok 7
 ex bank ["wd 1"] = err "overdraft"
 fold @bankstep bzero cs
```

### jsonsum
```aleph
fn jsonsum s:str -> res i64 str
 ex jsonsum "[1, 2, 3]" = ok 6
 ex isok jsonsum "[1, \"a\"]" = false
 let j try okor jparse s "bad"
 let xs try okor jlist j "bad"
 let ns try mapr lam x okor jint x "bad" xs
 ok sum ns
```

### median
```aleph
fn median xs:list i64 -> opt f64
 ex median [3 1 2] = some 2.0
 ex median empty i64 = none f64
 if == len xs 0 none f64
 let s sort xs
 let h / len s 2
 if == % len s 2 1 some tof else at s h 0
 some / + tof else at s - h 1 0 tof else at s h 0 2.0
```

### primes
```aleph
fn isprime n:i64 -> bool
 ex isprime 7 = true
 ex isprime 9 = false
 if < n 2 false
 == len filter lam d == % n d 0 range 2 n 0
fn primes n:i64 -> list i64
 ex primes 10 = [2 3 5 7]
 ex primes 1 = empty i64
 filter @isprime range 2 + n 1
```

### titlecase
```aleph
fn cap w:str -> str
 ex cap "wORLD" = "World"
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
 ex gcd 0 5 = 5
 if == b 0 a
 gcd b % a b
fn gcdlist xs:list i64 -> i64
 ex gcdlist [12 18 30] = 6
 ex gcdlist empty i64 = 0
 fold @gcd 0 xs
```

### kvdict
```aleph
fn kvzero -> res dict str i64 str
 ex isok kvzero = true
 ok dict str i64
fn kvstep acc:res dict str i64 str p:str -> res dict str i64 str
 ex isok kvstep kvzero "a" = false
 let d try acc
 let kv split p "="
 if != len kv 2 err "bad"
 let k else at kv 0 ""
 if == k "" err "bad"
 let v try okor int else at kv 1 "" "bad"
 ok put d k v
fn kvdict s:str -> res dict str i64 str
 ex kvdict "a=1,b=2" = ok put put dict str i64 "a" 1 "b" 2
 ex isok kvdict "a" = false
 if == s "" ok dict str i64
 fold @kvstep kvzero split s ","
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
fn walk g:dict str list str todo:list str seen:list str -> i64
 ex walk dict str list str empty str empty str = 0
 if == len todo 0 len seen
 let n else at todo 0 ""
 let rest slice todo 1 len todo
 if has index seen n walk g rest seen
 let nb else get g n empty str
 walk g cat rest nb push seen n
fn reachable g:dict str list str s:str -> i64
 ex reachable put put dict str list str "a" ["b"] "b" ["a" "c"] "a" = 3
 ex reachable dict str list str "x" = 1
 walk g [s] empty str
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
