### rpn

```aleph
fn isop t:str -> bool
 ex isop "+" = true
 ex isop "4" = false
 or == t "+" or == t "-" == t "*"
fn apply o:str a:i64 b:i64 -> i64
 ex apply "*" 2 3 = 6
 if == o "+" + a b
 if == o "-" - a b
 * a b
fn applyop st:list i64 o:str -> res list i64 str
 ex applyop [3 4] "+" = ok [7]
 ex isok applyop [3] "+" = false
 if < len st 2 err "bad"
 let b nth st - len st 1
 let a nth st - len st 2
 let keep slice st 0 - len st 2
 ok push keep apply o a b
fn rpnstep st:list i64 t:str -> res list i64 str
 ex rpnstep [3] "4" = ok [3 4]
 ex isok rpnstep [1] "+" = false
 if isop t applyop st t
 let v try okor int t "bad"
 ok push st v
fn rpnloop ts:list str st:list i64 -> res list i64 str
 ex rpnloop ["5" "2"] empty i64 = ok [5 2]
 ex rpnloop empty str [4] = ok [4]
 if == len ts 0 ok st
 let t nth ts 0
 let st2 try rpnstep st t
 let rest slice ts 1 len ts
 rpnloop rest st2
fn rpn s:str -> res i64 str
 ex rpn "3 4 + 2 *" = ok 14
 ex isok rpn "1 +" = false
 let ts fields s
 let st try rpnloop ts empty i64
 if == len st 1 ok nth st 0
 err "bad"
```

### merge

```aleph
fn merge a:list i64 b:list i64 -> list i64
 ex merge [1 4] [2 3 5] = [1 2 3 4 5]
 ex merge empty i64 empty i64 = empty i64
 if == len a 0 b
 if == len b 0 a
 let x nth a 0
 let y nth b 0
 let ra slice a 1 len a
 let rb slice b 1 len b
 if <= x y cat [x] merge ra b
 cat [y] merge a rb
```

### runs

```aleph
fn lead xs:list i64 x:i64 -> i64
 ex lead [1 1 2] 1 = 2
 ex lead empty i64 1 = 0
 if == len xs 0 0
 let h nth xs 0
 let t slice xs 1 len xs
 if == h x + 1 lead t x
 0
fn runs xs:list i64 -> list list i64
 ex runs [1 1 2 1] = [[1 1] [2] [1]]
 ex runs empty i64 = empty list i64
 if == len xs 0 empty list i64
 let x nth xs 0
 let n lead xs x
 let first slice xs 0 n
 let rest slice xs n len xs
 cat [first] runs rest
```

### toroman

```aleph
fn digit d:i64 one:str five:str ten:str -> str
 ex digit 4 "I" "V" "X" = "IV"
 ex digit 7 "X" "L" "C" = "LXX"
 if == d 0 ""
 if < d 4 repeat one d
 if == d 4 cat one five
 if == d 5 five
 let k - d 5
 if < d 9 cat five repeat one k
 cat one ten
fn toroman n:i64 -> str
 ex toroman 1994 = "MCMXCIV"
 ex toroman 3 = "III"
 let th repeat "M" / n 1000
 let hu digit % / n 100 10 "C" "D" "M"
 let te digit % / n 10 10 "X" "L" "C"
 let un digit % n 10 "I" "V" "X"
 let a cat th hu
 let b cat te un
 cat a b
```

### fromroman

```aleph
fn rval c:str -> i64
 ex rval "X" = 10
 ex rval "M" = 1000
 if == c "I" 1
 if == c "V" 5
 if == c "X" 10
 if == c "L" 50
 if == c "C" 100
 if == c "D" 500
 1000
fn romansum cs:list str -> i64
 ex romansum ["I" "V"] = 4
 ex romansum empty str = 0
 if == len cs 0 0
 let a rval nth cs 0
 if == len cs 1 a
 let b rval nth cs 1
 let rest slice cs 1 len cs
 let rest2 slice cs 2 len cs
 if < a b + - b a romansum rest2
 + a romansum rest
fn fromroman s:str -> i64
 ex fromroman "MCMXCIV" = 1994
 ex fromroman "III" = 3
 let cs split s ""
 romansum cs
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
fn best d:dict str i64 ws:list str -> str
 ex best put put dict str i64 "a" 2 "b" 2 ["a" "b"] = "a"
 let w nth ws 0
 if == len ws 1 w
 let rest slice ws 1 len ws
 let r best d rest
 let cw else get d w 0
 let cr else get d r 0
 if > cr cw r
 w
fn top d:dict str i64 k:i64 -> list str
 ex top put put dict str i64 "a" 1 "b" 2 1 = ["b"]
 if or < k 1 == size d 0 empty str
 let ks keys d
 let w best d ks
 let d2 del d w
 let k2 - k 1
 cat [w] top d2 k2
fn topn s:str k:i64 -> list str
 ex topn "b a b c a b" 2 = ["b" "a"]
 ex topn "" 3 = empty str
 let d fold lam a x put a x + else get a x 0 1 dict str i64 fields s
 top d k
```

### transpose

```aleph
fn transpose m:list list i64 -> list list i64
 ex transpose [[1 2 3] [4 5 6]] = [[1 4] [2 5] [3 6]]
 ex transpose empty list i64 = empty list i64
 if == len m 0 empty list i64
 let first nth m 0
 if == len first 0 empty list i64
 let heads map lam r nth r 0 m
 let tails map lam r slice r 1 len r m
 cat [heads] transpose tails
```

### brackets

```aleph
fn opener c:str -> str
 ex opener ")" = "("
 ex opener "x" = ""
 if == c ")" "("
 if == c "]" "["
 if == c "}" "{"
 ""
fn brrun cs:list str st:list str -> bool
 ex brrun ["(" ")"] empty str = true
 ex brrun ["(" "]"] empty str = false
 ex brrun empty str empty str = true
 if == len cs 0 == len st 0
 let c nth cs 0
 let rest slice cs 1 len cs
 if not contains "()[]{}" c brrun rest st
 let st2 push st c
 if contains "([{" c brrun rest st2
 if == len st 0 false
 let top nth st - len st 1
 let st3 slice st 0 - len st 1
 if == top opener c brrun rest st3
 false
fn brackets s:str -> bool
 ex brackets "([]{x})" = true
 ex brackets "(]" = false
 let cs split s ""
 brrun cs empty str
```

### wrap

```aleph
fn wrapl ws:list str cur:str w:i64 -> list str
 ex wrapl ["a" "bb"] "" 4 = ["a bb"]
 ex wrapl empty str "" 4 = empty str
 if and == len ws 0 == cur "" empty str
 if == len ws 0 [cur]
 let x nth ws 0
 let rest slice ws 1 len ws
 if == cur "" wrapl rest x w
 let grown cat cat cur " " x
 if <= len grown w wrapl rest grown w
 cat [cur] wrapl ws "" w
fn wrap s:str w:i64 -> list str
 ex wrap "a bb ccc d" 4 = ["a bb" "ccc" "d"]
 ex wrap "" 3 = empty str
 let ws fields s
 wrapl ws "" w
```

### bank

```aleph
fn step bal:i64 c:str -> res i64 str
 ex step 5 "dep 2" = ok 7
 ex step 0 "wd 1" = err "overdraft"
 let ts fields c
 if != len ts 2 err "bad"
 let n try okor int nth ts 1 "bad"
 if < n 0 err "bad"
 if == nth ts 0 "dep" ok + bal n
 if != nth ts 0 "wd" err "bad"
 if > n bal err "overdraft"
 ok - bal n
fn bankrun cs:list str bal:i64 -> res i64 str
 ex bankrun ["dep 10" "wd 3"] 0 = ok 7
 ex isok bankrun ["x"] 0 = false
 if == len cs 0 ok bal
 let c nth cs 0
 let rest slice cs 1 len cs
 let nb try step bal c
 bankrun rest nb
fn bank cs:list str -> res i64 str
 ex bank ["dep 10" "wd 3"] = ok 7
 ex bank ["wd 1"] = err "overdraft"
 bankrun cs 0
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
 let h / n 2
 if == % n 2 1 some tof nth s h
 let a tof nth s - h 1
 let b tof nth s h
 some / + a b 2.0
```

### primes

```aleph
fn nodiv n:i64 d:i64 -> bool
 ex nodiv 7 2 = true
 ex nodiv 9 2 = false
 if > * d d n true
 if == % n d 0 false
 nodiv n + d 1
fn isprime n:i64 -> bool
 ex isprime 7 = true
 ex isprime 9 = false
 ex isprime 1 = false
 if < n 2 false
 nodiv n 2
fn primes n:i64 -> list i64
 ex primes 10 = [2 3 5 7]
 ex primes 1 = empty i64
 let hi + n 1
 let xs range 2 hi
 filter @isprime xs
```

### titlecase

```aleph
fn cap w:str -> str
 ex cap "wORLD" = "World"
 ex cap "a" = "A"
 let cs split w ""
 let f upper nth cs 0
 let r join slice cs 1 len cs ""
 cat f lower r
fn titlecase s:str -> str
 ex titlecase "hello  wORLD" = "Hello World"
 ex titlecase "" = ""
 let ws fields s
 let caps map lam w cap w ws
 join caps " "
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
 fold lam a x gcd a x 0 xs
```

### kvdict

```aleph
fn kvrun ps:list str d:dict str i64 -> res dict str i64 str
 ex kvrun ["a=1"] dict str i64 = ok put dict str i64 "a" 1
 ex isok kvrun ["a"] dict str i64 = false
 if == len ps 0 ok d
 let p nth ps 0
 let kv split p "="
 if != len kv 2 err "bad"
 if == nth kv 0 "" err "bad"
 let v try okor int nth kv 1 "bad"
 let d2 put d nth kv 0 v
 kvrun slice ps 1 len ps d2
fn kvdict s:str -> res dict str i64 str
 ex kvdict "a=1,b=2" = ok put put dict str i64 "a" 1 "b" 2
 ex isok kvdict "a" = false
 if == s "" ok dict str i64
 let ps split s ","
 kvrun ps dict str i64
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
fn reach g:dict str list str stack:list str seen:dict str i64 -> i64
 ex reach dict str list str empty str dict str i64 = 0
 if == len stack 0 size seen
 let n nth stack 0
 let rest slice stack 1 len stack
 if has get seen n reach g rest seen
 let seen2 put seen n 1
 let nb else get g n empty str
 let nxt cat nb rest
 reach g nxt seen2
fn reachable g:dict str list str s:str -> i64
 ex reachable put put dict str list str "a" ["b"] "b" ["a" "c"] "a" = 3
 ex reachable dict str list str "x" = 1
 let st [s]
 let seen dict str i64
 reach g st seen
```

### secondmax

```aleph
fn secondmax xs:list i64 -> opt i64
 ex secondmax [3 1 3 2] = some 2
 ex secondmax [5 5] = none i64
 let u sort uniq xs
 if < len u 2 none i64
 some nth u - len u 2
```
