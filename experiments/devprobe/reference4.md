### rpn
```aleph
fn rp1 st:list i64 toks:list str -> res i64 str
 ex rp1 empty i64 ["1"] = ok 1
 if == len toks 0 if == len st 1 okor at st 0 "bad" err "bad"
 let t else at toks 0 ""
 let rest slice toks 1 len toks
 if not or == t "+" or == t "-" == t "*" rp1 push st try okor int t "bad" rest
 let n len st
 if < n 2 err "bad"
 let a else at st - n 2 0
 let b else at st - n 1 0
 rp1 push slice st 0 - n 2 if == t "+" + a b if == t "-" - a b * a b rest
fn rpn s:str -> res i64 str
 ex rpn "3 4 + 2 *" = ok 14
 rp1 empty i64 fields s
```

### merge
```aleph
fn merge a:list i64 b:list i64 -> list i64
 ex merge [1 4] [2 3 5] = [1 2 3 4 5]
 sort cat a b
```

### runs
```aleph
fn rn1 acc:list list i64 x:i64 -> list list i64
 ex rn1 [[1]] 1 = [[1 1]]
 let n len acc
 let last else at acc - n 1 empty i64
 if == else at last 0 - x 1 x push slice acc 0 - n 1 push last x
 push acc [x]
fn runs xs:list i64 -> list list i64
 ex runs [1 1 2 1] = [[1 1] [2] [1]]
 fold @rn1 empty list i64 xs
```

### toroman
```aleph
fn toroman n:i64 -> str
 ex toroman 1994 = "MCMXCIV"
 if == n 0 ""
 let vs [1000 900 500 400 100 90 50 40 10 9 5 4 1]
 let ss ["M" "CM" "D" "CD" "C" "XC" "L" "XL" "X" "IX" "V" "IV" "I"]
 let i else at filter lam j <= else at vs j 0 n range 0 13 0 0
 cat else at ss i "" toroman - n else at vs i 0
```

### fromroman
```aleph
fn rv c:str -> i64
 ex rv "X" = 10
 else get put put put put put put put dict str i64 "I" 1 "V" 5 "X" 10 "L" 50 "C" 100 "D" 500 "M" 1000 c 0
fn fromroman s:str -> i64
 ex fromroman "MCMXCIV" = 1994
 let vs map @rv split s ""
 sum map lam i if < else at vs i 0 else at vs + i 1 0 neg else at vs i 0 else at vs i 0 range 0 len vs
```

### evalexpr
```aleph
type expr | num v:i64 | add a:expr b:expr | mul a:expr b:expr
fn evalexpr e:expr -> i64
 ex evalexpr add num 2 mul num 3 num 4 = 14
 match e num v v add a b + evalexpr a evalexpr b mul a b * evalexpr a evalexpr b
```

### topn
```aleph
fn topn s:str k:i64 -> list str
 ex topn "b a b c a b" 2 = ["b" "a"]
 let ws fields s
 slice sortby lam w neg len filter lam x == x w ws sort uniq ws 0 k
```

### transpose
```aleph
fn transpose m:list list i64 -> list list i64
 ex transpose [[1 2 3] [4 5 6]] = [[1 4] [2 5] [3 6]]
 map lam j map lam r else at r j 0 m range 0 len else at m 0 empty i64
```

### brackets
```aleph
fn br1 st:list str c:str -> list str
 ex br1 ["("] ")" = empty str
 if == else at st 0 "" "!" st
 if contains "([{" c push st c
 if not contains ")]}" c st
 let want if == c ")" "(" if == c "]" "[" "{"
 if == else at st - len st 1 "" want slice st 0 - len st 1
 ["!"]
fn brackets s:str -> bool
 ex brackets "([]{x})" = true
 ex brackets "(]" = false
 == len fold @br1 empty str split s "" 0
```

### wrap
```aleph
fn wr1 w:i64 ls:list str x:str -> list str
 ex wr1 4 ["a"] "bb" = ["a bb"]
 let n len ls
 let last else at ls - n 1 ""
 if and > n 0 <= + + count last 1 count x w push slice ls 0 - n 1 cat cat last " " x
 push ls x
fn wrap s:str w:i64 -> list str
 ex wrap "a bb ccc d" 4 = ["a bb" "ccc" "d"]
 fold lam ls x wr1 w ls x empty str fields s
```

### bank
```aleph
fn bk1 b:i64 cs:list str -> res i64 str
 ex bk1 0 ["dep 5"] = ok 5
 if == len cs 0 ok b
 let p fields else at cs 0 ""
 let op else at p 0 ""
 let n try okor int else at p 1 "" "bad"
 if or or != len p 2 < n 0 and != op "dep" != op "wd" err "bad"
 let nb if == op "dep" + b n - b n
 if < nb 0 err "overdraft"
 bk1 nb slice cs 1 len cs
fn bank cs:list str -> res i64 str
 ex bank ["dep 10" "wd 3"] = ok 7
 ex bank ["wd 1"] = err "overdraft"
 bk1 0 cs
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
 let n len s
 some / tof + else at s / n 2 0 else at s / - n 1 2 0 2.0
```

### primes
```aleph
fn isp n:i64 -> bool
 ex isp 7 = true
 and >= n 2 == 0 len filter lam d == % n d 0 range 2 n
fn primes n:i64 -> list i64
 ex primes 10 = [2 3 5 7]
 filter @isp range 2 + n 1
```

### titlecase
```aleph
fn cap1 w:str -> str
 ex cap1 "hELLO" = "Hello"
 let cs split w ""
 cat upper else at cs 0 "" lower join slice cs 1 len cs ""
fn titlecase s:str -> str
 ex titlecase "hello  wORLD" = "Hello World"
 join map @cap1 fields s " "
```

### gcdlist
```aleph
fn gcd a:i64 b:i64 -> i64
 ex gcd 12 18 = 6
 if == b 0 a gcd b % a b
fn gcdlist xs:list i64 -> i64
 ex gcdlist [12 18 30] = 6
 fold @gcd 0 xs
```

### kvdict
```aleph
fn kv1 d:dict str i64 ps:list str -> res dict str i64 str
 ex kv1 dict str i64 ["a=1"] = ok put dict str i64 "a" 1
 if == len ps 0 ok d
 let kv split else at ps 0 "" "="
 if != len kv 2 err "bad"
 let v try okor int else at kv 1 "" "bad"
 kv1 put d else at kv 0 "" v slice ps 1 len ps
fn kvdict s:str -> res dict str i64 str
 ex kvdict "a=1,b=2" = ok put put dict str i64 "a" 1 "b" 2
 if == s "" ok dict str i64
 kv1 dict str i64 split s ","
```

### leaves
```aleph
type tree | leaf v:i64 | node l:tree r:tree
fn leaves t:tree -> list i64
 ex leaves node leaf 1 node leaf 2 leaf 3 = [1 2 3]
 match t leaf v [v] node l r cat leaves l leaves r
```

### reachable
```aleph
fn rc1 g:dict str list str seen:list str todo:list str -> list str
 ex rc1 dict str list str empty str ["a"] = ["a"]
 if == len todo 0 seen
 let x else at todo 0 ""
 let rest slice todo 1 len todo
 if has index seen x rc1 g seen rest
 rc1 g push seen x cat rest else get g x empty str
fn reachable g:dict str list str s:str -> i64
 ex reachable put put dict str list str "a" ["b"] "b" ["a" "c"] "a" = 3
 len rc1 g empty str [s]
```

### secondmax
```aleph
fn secondmax xs:list i64 -> opt i64
 ex secondmax [3 1 3 2] = some 2
 ex secondmax [5 5] = none i64
 at rev uniq sort xs 1
```
