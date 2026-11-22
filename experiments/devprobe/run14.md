# Aleph solutions

### rpn

```aleph
fn plus st:list i64 -> res list i64 str
 ex plus [1 2] = ok [3]
 if < len st 2 err "bad"
 let a else at st - len st 2 0
 let b else at st - len st 1 0
 ok push slice st 0 - len st 2 + a b

fn minus st:list i64 -> res list i64 str
 ex minus [5 3] = ok [2]
 if < len st 2 err "bad"
 let a else at st - len st 2 0
 let b else at st - len st 1 0
 ok push slice st 0 - len st 2 - a b

fn times st:list i64 -> res list i64 str
 ex times [3 4] = ok [12]
 if < len st 2 err "bad"
 let a else at st - len st 2 0
 let b else at st - len st 1 0
 ok push slice st 0 - len st 2 * a b

fn rpnloop toks:list str st:list i64 -> res list i64 str
 ex rpnloop ["1" "2" "+"] empty i64 = ok [3]
 if == len toks 0 ok st
 let t else at toks 0 ""
 let rest slice toks 1 len toks
 if == t "+" rpnloop rest try plus st
 if == t "-" rpnloop rest try minus st
 if == t "*" rpnloop rest try times st
 match int t ok v rpnloop rest push st v err e err "bad"

fn rpn s:str -> res i64 str
 ex rpn "3 4 + 2 *" = ok 14
 ex isok rpn "1 +" = false
 let st try rpnloop fields s empty i64
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
fn takeeq h:i64 t:list i64 -> list i64
 ex takeeq 1 [1 1 2] = [1 1]
 ex takeeq 9 [1] = empty i64
 if == len t 0 empty i64
 if == else at t 0 0 h cat [h] takeeq h slice t 1 len t
 empty i64

fn runs xs:list i64 -> list list i64
 ex runs [1 1 2 1] = [[1 1] [2] [1]]
 ex runs empty i64 = empty list i64
 if == len xs 0 empty list i64
 let h else at xs 0 0
 let t slice xs 1 len xs
 let run cat [h] takeeq h t
 cat [run] runs slice xs len run len xs
```

### toroman

```aleph
fn romdigit d:i64 one:str five:str ten:str -> str
 ex romdigit 4 "I" "V" "X" = "IV"
 ex romdigit 9 "I" "V" "X" = "IX"
 if == d 0 ""
 if == d 1 one
 if == d 2 cat one one
 if == d 3 cat one cat one one
 if == d 4 cat one five
 if == d 5 five
 if == d 6 cat five one
 if == d 7 cat five cat one one
 if == d 8 cat five cat one cat one one
 cat one ten

fn toroman n:i64 -> str
 ex toroman 1994 = "MCMXCIV"
 ex toroman 3 = "III"
 let th / n 1000
 let h / % n 1000 100
 let te / % n 100 10
 let o % n 10
 let a repeat "M" th
 let b romdigit h "C" "D" "M"
 let c romdigit te "X" "L" "C"
 let e romdigit o "I" "V" "X"
 cat cat a b cat c e
```

### fromroman

```aleph
fn rval c:str -> i64
 ex rval "I" = 1
 ex rval "M" = 1000
 if == c "I" 1
 if == c "V" 5
 if == c "X" 10
 if == c "L" 50
 if == c "C" 100
 if == c "D" 500
 1000

fn rsum vs:list i64 -> i64
 ex rsum [1 5] = 4
 ex rsum [1000 100 1000 10 100 1 5] = 1994
 if == len vs 0 0
 if == len vs 1 else at vs 0 0
 let a else at vs 0 0
 let b else at vs 1 0
 if < a b + neg a rsum slice vs 1 len vs
 + a rsum slice vs 1 len vs

fn fromroman s:str -> i64
 ex fromroman "MCMXCIV" = 1994
 ex fromroman "III" = 3
 rsum map @rval split s ""
```

### evalexpr

```aleph
type expr | num v:i64 | add a:expr b:expr | mul a:expr b:expr
fn evalexpr e:expr -> i64
 ex evalexpr add num 2 mul num 3 num 4 = 14
 ex evalexpr num 5 = 5
 match e num n n add x y + evalexpr x evalexpr y mul x y * evalexpr x evalexpr y
```

### topn

```aleph
fn topn s:str k:i64 -> list str
 ex topn "b a b c a b" 2 = ["b" "a"]
 ex topn "" 3 = empty str
 let ws fields s
 let d fold lam acc w put acc w + else get acc w 0 1 dict str i64 ws
 let u sort uniq ws
 let ranked sortby lam w neg else get d w 0 u
 slice ranked 0 k
```

### transpose

```aleph
fn col m:list list i64 j:i64 -> list i64
 ex col [[1 2] [3 4]] 0 = [1 3]
 map lam r else at r j 0 m

fn transpose m:list list i64 -> list list i64
 ex transpose [[1 2 3] [4 5 6]] = [[1 4] [2 5] [3 6]]
 ex transpose empty list i64 = empty list i64
 if == len m 0 empty list i64
 let w len else at m 0 empty i64
 if == w 0 empty list i64
 map lam j col m j range 0 w
```

### brackets

```aleph
fn brkmatch c:str rest:list str st:list str -> bool
 ex brkmatch ")" [] ["("] = false
 ex brkmatch ")" [] [")"] = true
 if == len st 0 false
 if == else at st - len st 1 "" c brk rest slice st 0 - len st 1
 false

fn brk cs:list str st:list str -> bool
 ex brk ["(" ")"] empty str = true
 ex brk ["("] empty str = false
 if == len cs 0 == len st 0
 let c else at cs 0 ""
 let rest slice cs 1 len cs
 if == c "(" brk rest push st ")"
 if == c "[" brk rest push st "]"
 if == c "{" brk rest push st "}"
 if or == c ")" or == c "]" == c "}" brkmatch c rest st
 brk rest st

fn brackets s:str -> bool
 ex brackets "([]{x})" = true
 ex brackets "(]" = false
 brk split s "" empty str
```

### wrap

```aleph
fn wrapgo ws:list str w:i64 cur:str acc:list str -> list str
 ex wrapgo ["a" "bb"] 4 "" empty str = ["a bb"]
 ex wrapgo ["cccc" "a"] 3 "" empty str = ["cccc" "a"]
 if == len ws 0 if == len cur 0 acc cat acc [cur]
 let x else at ws 0 ""
 let rest slice ws 1 len ws
 if == len cur 0 wrapgo rest w x acc
 if <= + + len cur 1 len x w wrapgo rest w cat cur cat " " x acc
 wrapgo rest w x cat acc [cur]

fn wrap s:str w:i64 -> list str
 ex wrap "a bb ccc d" 4 = ["a bb" "ccc" "d"]
 ex wrap "" 3 = empty str
 wrapgo fields s w "" empty str
```

### bank

```aleph
fn amt s:str -> opt i64
 ex amt "10" = some 10
 ex amt "x" = none i64
 match int s ok v some v err e none i64

fn bankgo cs:list str bal:i64 -> res i64 str
 ex bankgo ["dep 10" "wd 3"] 0 = ok 7
 ex bankgo ["wd 1"] 0 = err "overdraft"
 if == len cs 0 ok bal
 let c else at cs 0 ""
 let fs fields c
 if != len fs 2 err "bad"
 let op else at fs 0 ""
 let v else amt else at fs 1 "" -1
 if < v 0 err "bad"
 let rest slice cs 1 len cs
 if == op "dep" bankgo rest + bal v
 if == op "wd" if < - bal v 0 err "overdraft" bankgo rest - bal v
 err "bad"

fn bank cs:list str -> res i64 str
 ex bank ["dep 10" "wd 3"] = ok 7
 ex bank ["wd 1"] = err "overdraft"
 bankgo cs 0
```

### jsonsum

```aleph
fn jsonsum s:str -> res i64 str
 ex jsonsum "[1, 2, 3]" = ok 6
 ex isok jsonsum "[1, \"a\"]" = false
 let j try okor jparse s "bad"
 let arr try okor jlist j "bad"
 let ns try mapr lam e okor jint e "bad" arr
 ok sum ns
```

### median

```aleph
fn median xs:list i64 -> opt f64
 ex median [3 1 2] = some 2.0
 ex median empty i64 = none f64
 let s sort xs
 let n len s
 if == n 0 none f64
 if == % n 2 1 some tof else at s / n 2 0
 let a else at s - / n 2 1 0
 let b else at s / n 2 0
 some / tof + a b 2.0
```

### primes

```aleph
fn isprime p:i64 -> bool
 ex isprime 7 = true
 ex isprime 9 = false
 ex isprime 2 = true
 if < p 2 false
 testdiv p 2

fn testdiv p:i64 d:i64 -> bool
 ex testdiv 7 2 = true
 ex testdiv 9 2 = false
 if > * d d p true
 if == % p d 0 false
 testdiv p + d 1

fn primes n:i64 -> list i64
 ex primes 10 = [2 3 5 7]
 ex primes 1 = empty i64
 if < n 2 empty i64
 filter @isprime range 2 + n 1
```

### titlecase

```aleph
fn capword w:str -> str
 ex capword "wORLD" = "World"
 ex capword "" = ""
 let cs split w ""
 if == len cs 0 ""
 cat upper else at cs 0 "" join slice cs 1 len cs ""

fn titlecase s:str -> str
 ex titlecase "hello  wORLD" = "Hello World"
 ex titlecase "" = ""
 join map @capword fields s " "
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
fn num s:str -> opt i64
 ex num "2" = some 2
 ex num "x" = none i64
 match int s ok v some v err e none i64

fn kvgo ps:list str d:dict str i64 -> res dict str i64 str
 ex kvgo ["a=1"] dict str i64 = ok put dict str i64 "a" 1
 ex isok kvgo ["a"] dict str i64 = false
 if == len ps 0 ok d
 let p else at ps 0 ""
 let parts split p "="
 if != len parts 2 err "bad"
 let k else at parts 0 ""
 if == len k 0 err "bad"
 let vopt num else at parts 1 ""
 if has vopt kvgo slice ps 1 len ps put d k else vopt 0
 err "bad"

fn kvdict s:str -> res dict str i64 str
 ex kvdict "a=1,b=2" = ok put put dict str i64 "a" 1 "b" 2
 ex isok kvdict "a" = false
 if == len s 0 ok dict str i64
 kvgo split s "," dict str i64
```

### leaves

```aleph
type tree | leaf v:i64 | node l:tree r:tree
fn leaves t:tree -> list i64
 ex leaves node leaf 1 node leaf 2 leaf 3 = [1 2 3]
 ex leaves leaf 4 = [4]
 match t leaf n [n] node a b cat leaves a leaves b
```

### reachable

```aleph
fn visitall g:dict str list str ns:list str vs:dict str bool -> dict str bool
 ex visitall dict str list str empty str dict str bool = dict str bool
 if == len ns 0 vs
 let m else at ns 0 ""
 visitall g slice ns 1 len ns visit g m vs

fn visit g:dict str list str n:str vs:dict str bool -> dict str bool
 ex visit dict str list str "a" dict str bool = put dict str bool "a" true
 if has get vs n vs
 visitall g else get g n empty str put vs n true

fn reachable g:dict str list str s:str -> i64
 ex reachable put put dict str list str "a" ["b"] "b" ["a" "c"] "a" = 3
 ex reachable dict str list str "x" = 1
 size visit g s dict str bool
```

### secondmax

```aleph
fn secondmax xs:list i64 -> opt i64
 ex secondmax [3 1 3 2] = some 2
 ex secondmax [5 5] = none i64
 let u rev sort uniq xs
 at u 1
```
