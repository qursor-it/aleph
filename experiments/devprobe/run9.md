### rpn

```aleph
fn binop st:list i64 t:str -> res list i64 str
 ex isok binop [2 3] "+" = true
 let n len st
 if < n 2 err "bad"
 let b else at st - n 1 0
 let a else at st - n 2 0
 let r if == t "+" + a b if == t "-" - a b * a b
 ok push slice st 0 - n 2 r
fn step st:res list i64 str t:str -> res list i64 str
 ex isok step ok [2] "3" = true
 let cur try st
 if or == t "+" or == t "-" == t "*" binop cur t
 if not isok int t err "bad"
 ok push cur else int t 0
fn rpn s:str -> res i64 str
 ex rpn "3 4 + 2 *" = ok 14
 ex isok rpn "1 +" = false
 let st try fold @step ok empty i64 fields s
 if == len st 1 ok else at st 0 0 err "bad"
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
 if <= x y cat [x] merge slice a 1 len a b
 cat [y] merge a slice b 1 len b
```

### runs

```aleph
fn eqlen xs:list i64 x:i64 -> i64
 ex eqlen [1 1 2] 1 = 2
 if == len xs 0 0
 if == else at xs 0 0 x + 1 eqlen slice xs 1 len xs x
 0
fn runs xs:list i64 -> list list i64
 ex runs [1 1 2 1] = [[1 1] [2] [1]]
 ex runs empty i64 = empty list i64
 if == len xs 0 empty list i64
 let x else at xs 0 0
 let n eqlen xs x
 cat [slice xs 0 n] runs slice xs n len xs
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
 if >= n 1 cat "I" toroman - n 1
 ""
```

### fromroman

```aleph
fn rval c:str -> i64
 ex rval "M" = 1000
 if == c "I" 1
 if == c "V" 5
 if == c "X" 10
 if == c "L" 50
 if == c "C" 100
 if == c "D" 500
 if == c "M" 1000
 0
fn fromr cs:list str -> i64
 ex fromr ["X" "I"] = 11
 if == len cs 0 0
 let a rval else at cs 0 ""
 let b rval else at cs 1 ""
 if < a b + neg a fromr slice cs 1 len cs
 + a fromr slice cs 1 len cs
fn fromroman s:str -> i64
 ex fromroman "MCMXCIV" = 1994
 ex fromroman "III" = 3
 fromr split s ""
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
fn tally d:dict str i64 ws:list str -> dict str i64
 ex size tally dict str i64 ["a" "a" "b"] = 2
 if == len ws 0 d
 let w else at ws 0 ""
 let n else get d w 0
 tally put d w + n 1 slice ws 1 len ws
fn topn s:str k:i64 -> list str
 ex topn "b a b c a b" 2 = ["b" "a"]
 ex topn "" 3 = empty str
 if == s "" empty str
 let ws fields s
 let d tally dict str i64 ws
 let byw sort keys d
 let byc sortby lam w neg else get d w 0 byw
 slice byc 0 k
```

### transpose

```aleph
fn col m:list list i64 j:i64 -> list i64
 ex col [[1 2] [3 4]] 1 = [2 4]
 map lam r else at r j 0 m
fn transpose m:list list i64 -> list list i64
 ex transpose [[1 2 3] [4 5 6]] = [[1 4] [2 5] [3 6]]
 ex transpose empty list i64 = empty list i64
 if == len m 0 empty list i64
 let first else at m 0 empty i64
 let cols len first
 if == cols 0 empty list i64
 map lam j col m j range 0 cols
```

### brackets

```aleph
fn closeok st:list str c:str rest:list str -> bool
 ex closeok [")"] ")" empty str = true
 if == len st 0 false
 if != else at st - len st 1 "" c false
 matchbr rest slice st 0 - len st 1
fn matchbr cs:list str st:list str -> bool
 ex matchbr split "(x)" "" empty str = true
 if == len cs 0 == len st 0
 let c else at cs 0 ""
 let rest slice cs 1 len cs
 if == c "(" matchbr rest push st ")"
 if == c "[" matchbr rest push st "]"
 if == c "{" matchbr rest push st "}"
 if or == c ")" or == c "]" == c "}" closeok st c rest
 matchbr rest st
fn brackets s:str -> bool
 ex brackets "([]{x})" = true
 ex brackets "(]" = false
 matchbr split s "" empty str
```

### wrap

```aleph
fn pushacc cur:str acc:list str -> list str
 ex pushacc "" empty str = empty str
 if == cur "" acc
 push acc cur
fn wrapgo ws:list str w:i64 cur:str acc:list str -> list str
 ex wrapgo ["a" "bb"] 4 "" empty str = ["a bb"]
 if == len ws 0 pushacc cur acc
 let x else at ws 0 ""
 let rest slice ws 1 len ws
 if == cur "" wrapgo rest w x acc
 if <= + + len cur 1 len x w wrapgo rest w cat cat cur " " x acc
 wrapgo rest w x push acc cur
fn wrap s:str w:i64 -> list str
 ex wrap "a bb ccc d" 4 = ["a bb" "ccc" "d"]
 ex wrap "" 3 = empty str
 if == s "" empty str
 wrapgo fields s w "" empty str
```

### bank

```aleph
fn bankgo cs:list str bal:i64 -> res i64 str
 ex bankgo ["dep 5"] 0 = ok 5
 if == len cs 0 ok bal
 let c else at cs 0 ""
 let ts fields c
 if != len ts 2 err "bad"
 let op else at ts 0 ""
 let ns else at ts 1 ""
 if not isok int ns err "bad"
 let n else int ns 0
 if < n 0 err "bad"
 if == op "dep" bankgo slice cs 1 len cs + bal n
 if == op "wd" if < bal n err "overdraft" bankgo slice cs 1 len cs - bal n
 err "bad"
fn bank cs:list str -> res i64 str
 ex bank ["dep 10" "wd 3"] = ok 7
 ex bank ["wd 1"] = err "overdraft"
 bankgo cs 0
```

### jsonsum

```aleph
fn sumint xs:list json -> res i64 str
 ex sumint [json 1 json 2] = ok 3
 if == len xs 0 ok 0
 let x else at xs 0 json nil
 if not has jint x err "bad"
 let v else jint x 0
 let rest try sumint slice xs 1 len xs
 ok + v rest
fn jsonsum s:str -> res i64 str
 ex jsonsum "[1, 2, 3]" = ok 6
 ex isok jsonsum "[1, \"a\"]" = false
 let r jparse s
 if not isok r err "bad"
 let j else r json nil
 if not has jlist j err "bad"
 let xs else jlist j empty json
 sumint xs
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
fn nodiv n:i64 d:i64 -> bool
 ex nodiv 9 2 = false
 if > * d d n true
 if == % n d 0 false
 nodiv n + d 1
fn isprime n:i64 -> bool
 ex isprime 2 = true
 ex isprime 4 = false
 if < n 2 false
 nodiv n 2
fn primes n:i64 -> list i64
 ex primes 10 = [2 3 5 7]
 ex primes 1 = empty i64
 if < n 2 empty i64
 filter @isprime range 2 + n 1
```

### titlecase

```aleph
fn capw w:str -> str
 ex capw "wORLD" = "World"
 let cs split w ""
 if == len cs 0 ""
 let f else at cs 0 ""
 let rest slice cs 1 len cs
 cat upper f lower join rest ""
fn titlecase s:str -> str
 ex titlecase "hello  wORLD" = "Hello World"
 ex titlecase "" = ""
 if == s "" ""
 join map @capw fields s " "
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
fn kvparse d:dict str i64 parts:list str -> res dict str i64 str
 ex isok kvparse dict str i64 ["a=1"] = true
 if == len parts 0 ok d
 let p else at parts 0 ""
 let eq split p "="
 if != len eq 2 err "bad"
 let k else at eq 0 ""
 let v else at eq 1 ""
 if == k "" err "bad"
 if not isok int v err "bad"
 kvparse put d k else int v 0 slice parts 1 len parts
fn kvdict s:str -> res dict str i64 str
 ex kvdict "a=1,b=2" = ok put put dict str i64 "a" 1 "b" 2
 ex isok kvdict "a" = false
 if == s "" ok dict str i64
 kvparse dict str i64 split s ","
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
fn reach g:dict str list str todo:list str seen:list str -> list str
 ex len reach put dict str list str "a" ["a"] ["a"] empty str = 1
 if == len todo 0 seen
 let n else at todo 0 ""
 let rest slice todo 1 len todo
 if has index seen n reach g rest seen
 let nb else get g n empty str
 reach g cat rest nb push seen n
fn reachable g:dict str list str s:str -> i64
 ex reachable put put dict str list str "a" ["b"] "b" ["a" "c"] "a" = 3
 ex reachable dict str list str "x" = 1
 len reach g [s] empty str
```

### secondmax

```aleph
fn secondmax xs:list i64 -> opt i64
 ex secondmax [3 1 3 2] = some 2
 ex secondmax [5 5] = none i64
 let u uniq sort xs
 let n len u
 if < n 2 none i64
 some else at u - n 2 0
```
