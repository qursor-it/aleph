### rpn

```aleph
fn start -> res list i64 str
 ex isok start = true
 ok empty i64
fn applyop st:list i64 op:str -> res list i64 str
 ex applyop [3 4] "+" = ok [7]
 ex applyop [6 3] "-" = ok [3]
 ex applyop [2 3] "*" = ok [6]
 ex isok applyop [3] "+" = false
 if < len st 2 err "bad"
 let a else at st - len st 2 0
 let b else at st - len st 1 0
 let v if == op "+" + a b if == op "-" - a b * a b
 ok push slice st 0 - len st 2 v
fn pushnum st:list i64 tok:str -> res list i64 str
 ex pushnum [1] "2" = ok [1 2]
 ex isok pushnum [1] "x" = false
 let n try okor int tok "bad"
 ok push st n
fn step acc:res list i64 str tok:str -> res list i64 str
 ex step ok [3 4] "+" = ok [7]
 ex step ok [3] "4" = ok [3 4]
 let st try acc
 let r if or or == tok "+" == tok "*" == tok "-" applyop st tok pushnum st tok
 r
fn rpn s:str -> res i64 str
 ex rpn "3 4 + 2 *" = ok 14
 ex isok rpn "1 +" = false
 let st try fold @step start fields s
 if != len st 1 err "bad"
 ok else at st 0 0
```

### merge

```aleph
fn merge a:list i64 b:list i64 -> list i64
 ex merge [1 4] [2 3 5] = [1 2 3 4 5]
 ex merge empty i64 empty i64 = empty i64
 ex merge [2] [1] = [1 2]
 if == len a 0 b
 if == len b 0 a
 let x else at a 0 0
 let y else at b 0 0
 let r if <= x y cat [x] merge slice a 1 len a b cat [y] merge a slice b 1 len b
 r
```

### runs

```aleph
fn runlen xs:list i64 x:i64 -> i64
 ex runlen [1 1 2] 1 = 2
 ex runlen empty i64 1 = 0
 if == len xs 0 0
 let y else at xs 0 x
 if != y x 0
 + 1 runlen slice xs 1 len xs x
fn runs xs:list i64 -> list list i64
 ex runs [1 1 2 1] = [[1 1] [2] [1]]
 ex runs empty i64 = empty list i64
 if == len xs 0 empty list i64
 let x else at xs 0 0
 let n runlen xs x
 cat [slice xs 0 n] runs slice xs n len xs
```

### toroman

```aleph
fn digit d:i64 one:str five:str ten:str -> str
 ex digit 4 "I" "V" "X" = "IV"
 ex digit 9 "X" "L" "C" = "XC"
 ex digit 0 "I" "V" "X" = ""
 if == d 9 cat one ten
 if >= d 5 cat five repeat one - d 5
 if == d 4 cat one five
 repeat one d
fn toroman n:i64 -> str
 ex toroman 1994 = "MCMXCIV"
 ex toroman 3 = "III"
 let th repeat "M" / n 1000
 let h digit % / n 100 10 "C" "D" "M"
 let te digit % / n 10 10 "X" "L" "C"
 let o digit % n 10 "I" "V" "X"
 cat cat cat th h te o
```

### fromroman

```aleph
fn rval c:str -> i64
 ex rval "V" = 5
 ex rval "M" = 1000
 if == c "I" 1
 if == c "V" 5
 if == c "X" 10
 if == c "L" 50
 if == c "C" 100
 if == c "D" 500
 if == c "M" 1000
 0
fn sumroman cs:list str -> i64
 ex sumroman split "IV" "" = 4
 ex sumroman empty str = 0
 if == len cs 0 0
 let c else at cs 0 ""
 let v rval c
 let rest slice cs 1 len cs
 let nv if == len rest 0 0 rval else at rest 0 ""
 if < v nv + - nv v sumroman slice rest 1 len rest
 + v sumroman rest
fn fromroman s:str -> i64
 ex fromroman "MCMXCIV" = 1994
 ex fromroman "III" = 3
 sumroman split s ""
```

### evalexpr

```aleph
type expr | num v:i64 | add a:expr b:expr | mul a:expr b:expr
fn evalexpr e:expr -> i64
 ex evalexpr add num 2 mul num 3 num 4 = 14
 ex evalexpr num 5 = 5
 match e num x x add x y + evalexpr x evalexpr y mul x y * evalexpr x evalexpr y
```

### topn

```aleph
type wc w:str c:i64
fn bump acc:list wc w:str -> list wc
 ex bump empty wc "a" = [wc "a" 1]
 ex bump [wc "a" 1] "a" = [wc "a" 2]
 if == len acc 0 push acc wc w 1
 let last else at acc - len acc 1 wc "" 0
 if == .w last w push slice acc 0 - len acc 1 wc w + .c last 1
 push acc wc w 1
fn topn s:str k:i64 -> list str
 ex topn "b a b c a b" 2 = ["b" "a"]
 ex topn "" 3 = empty str
 let counts fold @bump empty wc sort fields s
 let bycount sortby lam x neg .c x counts
 slice map lam x .w x bycount 0 k
```

### transpose

```aleph
fn transpose m:list list i64 -> list list i64
 ex transpose [[1 2 3] [4 5 6]] = [[1 4] [2 5] [3 6]]
 ex transpose empty list i64 = empty list i64
 if == len m 0 empty list i64
 let r0 else at m 0 empty i64
 if == len r0 0 empty list i64
 let hs map lam r else at r 0 0 m
 let ts map lam r slice r 1 len r m
 cat [hs] transpose ts
```

### brackets

```aleph
fn pair c:str -> str
 ex pair ")" = "("
 ex pair "x" = ""
 if == c ")" "("
 if == c "]" "["
 if == c "}" "{"
 ""
fn step st:list str c:str -> list str
 ex step empty str "(" = ["("]
 ex step ["(" "["] "]" = ["("]
 ex step ["("] "]" = ["(" "!"]
 ex step empty str "x" = empty str
 if or or == c "(" == c "[" == c "{" push st c
 let p pair c
 if == p "" st
 if == len st 0 push st "!"
 let top else at st - len st 1 "!"
 let r if == top p slice st 0 - len st 1 push st "!"
 r
fn brackets s:str -> bool
 ex brackets "([]{x})" = true
 ex brackets "(]" = false
 let st fold @step empty str split s ""
 == len st 0
```

### wrap

```aleph
fn wrapgo ws:list str w:i64 cur:str -> list str
 ex wrapgo empty str 4 "" = empty str
 ex wrapgo ["d"] 4 "ccc" = ["ccc" "d"]
 if and == len ws 0 == cur "" empty str
 if == len ws 0 [cur]
 let x else at ws 0 ""
 let rest slice ws 1 len ws
 if == cur "" wrapgo rest w x
 let cand cat cat cur " " x
 if <= len cand w wrapgo rest w cand
 cat [cur] wrapgo ws w ""
fn wrap s:str w:i64 -> list str
 ex wrap "a bb ccc d" 4 = ["a bb" "ccc" "d"]
 ex wrap "" 3 = empty str
 wrapgo fields s w ""
```

### bank

```aleph
fn applycmd bal:i64 c:str -> res i64 str
 ex applycmd 10 "dep 5" = ok 15
 ex applycmd 3 "wd 1" = ok 2
 ex isok applycmd 3 "wd 5" = false
 ex isok applycmd 3 "xx" = false
 let parts fields c
 if != len parts 2 err "bad"
 let op else at parts 0 ""
 let n try okor int else at parts 1 "" "bad"
 if < n 0 err "bad"
 if == op "dep" ok + bal n
 if != op "wd" err "bad"
 if > n bal err "overdraft"
 ok - bal n
fn bstep acc:res i64 str c:str -> res i64 str
 ex bstep ok 5 "dep 1" = ok 6
 ex isok bstep err "x" "dep 1" = false
 let b try acc
 applycmd b c
fn zero -> res i64 str
 ex isok zero = true
 ok 0
fn bank cs:list str -> res i64 str
 ex bank ["dep 10" "wd 3"] = ok 7
 ex bank ["wd 1"] = err "overdraft"
 fold @bstep zero cs
```

### jsonsum

```aleph
fn jsonsum s:str -> res i64 str
 ex jsonsum "[1, 2, 3]" = ok 6
 ex isok jsonsum "[1, \"a\"]" = false
 let j try okor jparse s "bad"
 let xs try okor jlist j "bad"
 let ns try mapr lam e okor jint e "bad" xs
 ok sum ns
```

### median

```aleph
fn median xs:list i64 -> opt f64
 ex median [3 1 2] = some 2.0
 ex median empty i64 = none f64
 if == len xs 0 none f64
 let s sort xs
 let n len xs
 let h / n 2
 let hi tof else at s h 0
 if == % n 2 1 some hi
 let lo tof else at s - h 1 0
 some / + lo hi 2.0
```

### primes

```aleph
fn nodiv k:i64 d:i64 -> bool
 ex nodiv 9 3 = false
 ex nodiv 7 2 = true
 if > * d d k true
 if == % k d 0 false
 nodiv k + d 1
fn isprime k:i64 -> bool
 ex isprime 7 = true
 ex isprime 9 = false
 ex isprime 1 = false
 if < k 2 false
 nodiv k 2
fn primes n:i64 -> list i64
 ex primes 10 = [2 3 5 7]
 ex primes 1 = empty i64
 filter lam k isprime k range 2 + n 1
```

### titlecase

```aleph
fn cap w:str -> str
 ex cap "hELLO" = "Hello"
 ex cap "" = ""
 let cs split w ""
 let f else at cs 0 ""
 let rest join slice cs 1 len cs ""
 cat upper f lower rest
fn titlecase s:str -> str
 ex titlecase "hello  wORLD" = "Hello World"
 ex titlecase "" = ""
 join map lam w cap w fields s " "
```

### gcdlist

```aleph
fn gcd a:i64 b:i64 -> i64
 ex gcd 12 18 = 6
 ex gcd 7 0 = 7
 if == b 0 a
 gcd b % a b
fn gcdlist xs:list i64 -> i64
 ex gcdlist [12 18 30] = 6
 ex gcdlist empty i64 = 0
 fold @gcd 0 xs
```

### kvdict

```aleph
fn kvempty -> res dict str i64 str
 ex isok kvempty = true
 ok dict str i64
fn kvstep acc:res dict str i64 str pair:str -> res dict str i64 str
 ex isok kvstep kvempty "a" = false
 let d try acc
 let kv split pair "="
 if != len kv 2 err "bad"
 let k else at kv 0 ""
 if == k "" err "bad"
 let v try okor int else at kv 1 "" "bad"
 ok put d k v
fn kvdict s:str -> res dict str i64 str
 ex kvdict "a=1,b=2" = ok put put dict str i64 "a" 1 "b" 2
 ex isok kvdict "a" = false
 if == s "" kvempty
 fold @kvstep kvempty split s ","
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
fn walk g:dict str list str stack:list str seen:list str -> list str
 ex walk dict str list str empty str empty str = empty str
 if == len stack 0 seen
 let w else at stack - len stack 1 ""
 let rest slice stack 0 - len stack 1
 if has index seen w walk g rest seen
 let ns else get g w empty str
 walk g cat rest ns push seen w
fn reachable g:dict str list str s:str -> i64
 ex reachable put put dict str list str "a" ["b"] "b" ["a" "c"] "a" = 3
 ex reachable dict str list str "x" = 1
 len walk g [s] empty str
```

### secondmax

```aleph
fn secondmax xs:list i64 -> opt i64
 ex secondmax [3 1 3 2] = some 2
 ex secondmax [5 5] = none i64
 let u sort uniq xs
 if < len u 2 none i64
 some else at u - len u 2 0
```
