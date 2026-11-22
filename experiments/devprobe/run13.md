### rpn
```aleph
fn rpnstep acc:res list i64 str t:str -> res list i64 str
 ex rpnstep ok [1] "1" = ok [1 1]
 let st try acc
 rpntok st t
fn rpnnum st:list i64 t:str -> res list i64 str
 ex rpnnum empty i64 "5" = ok [5]
 ex rpnnum empty i64 "x" = err "bad"
 let n try okor int t "bad"
 ok push st n
fn rpnbin st:list i64 op:str -> res list i64 str
 ex rpnbin [1 2] "+" = ok [3]
 ex rpnbin [1] "+" = err "bad"
 let n len st
 let oa at st - n 2
 let ob at st - n 1
 match oa some a match ob some b ok push slice st 0 - n 2 if == op "+" + a b if == op "-" - a b * a b none err "bad" none err "bad"
fn rpntok st:list i64 t:str -> res list i64 str
 ex rpntok [1 2] "+" = ok [3]
 ex rpntok empty i64 "5" = ok [5]
 if == t "+" rpnbin st "+" if == t "-" rpnbin st "-" if == t "*" rpnbin st "*" rpnnum st t
fn rpn s:str -> res i64 str
 ex rpn "3 4 + 2 *" = ok 14
 ex isok rpn "1 +" = false
 let st try fold @rpnstep ok empty i64 fields s
 if == len st 1 ok else at st 0 0 err "bad"
```

### merge
```aleph
fn merge a:list i64 b:list i64 -> list i64
 ex merge [1 4] [2 3 5] = [1 2 3 4 5]
 ex merge empty i64 empty i64 = empty i64
 let x else at a 0 0
 let y else at b 0 0
 if == len a 0 b if == len b 0 a if <= x y push merge slice a 1 len a b y push a merge b slice b 1 len b
```

### runs
```aleph
fn runlen xs:list i64 h:i64 -> i64
 ex runlen [1 1 2] 1 = 2
 ex runlen empty i64 0 = 0
 let c else at xs 0 0
 if == len xs 0 0 if == c h + 1 runlen slice xs 1 len xs h 0
fn runs xs:list i64 -> list list i64
 ex runs [1 1 2 1] = [[1 1] [2] [1]]
 ex runs empty i64 = empty list i64
 let n runlen xs else at xs 0 0
 if == len xs 0 empty list i64 push runs slice xs n len xs slice xs 0 n
```

### toroman
```aleph
fn toroman n:i64 -> str
 ex toroman 1994 = "MCMXCIV"
 ex toroman 3 = "III"
 if >= n 1000 cat "M" toroman - n 1000 if >= n 900 cat "CM" toroman - n 900 if >= n 500 cat "D" toroman - n 500 if >= n 400 cat "CD" toroman - n 400 if >= n 100 cat "C" toroman - n 100 if >= n 90 cat "XC" toroman - n 90 if >= n 50 cat "L" toroman - n 50 if >= n 40 cat "XL" toroman - n 40 if >= n 10 cat "X" toroman - n 10 if >= n 9 cat "IX" toroman - n 9 if >= n 5 cat "V" toroman - n 5 if >= n 4 cat "IV" toroman - n 4 if >= n 1 cat "I" toroman - n 1 ""
```

### fromroman
```aleph
type rstate total:i64 prev:i64
fn rval c:str -> i64
 ex rval "M" = 1000
 if == c "I" 1 if == c "V" 5 if == c "X" 10 if == c "L" 50 if == c "C" 100 if == c "D" 500 if == c "M" 1000 0
fn romstep a:rstate c:str -> rstate
 ex romstep rstate 0 0 "X" = rstate 10 10
 let v rval c
 if > v .prev a rstate + .total a - v * 2 .prev a v rstate + .total a v v
fn fromroman s:str -> i64
 ex fromroman "MCMXCIV" = 1994
 ex fromroman "III" = 3
 .total fold @romstep rstate 0 0 split s ""
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
fn cnt d:dict str i64 w:str -> dict str i64
 ex size cnt dict str i64 "a" = 1
 put d w + 1 else get d w 0
fn topn s:str k:i64 -> list str
 ex topn "b a b c a b" 2 = ["b" "a"]
 ex topn "" 3 = empty str
 let d fold @cnt dict str i64 fields s
 let ks keys d
 let ordered sortby lam w neg else get d w 0 ks
 slice ordered 0 k
```

### transpose
```aleph
fn col m:list list i64 i:i64 -> list i64
 ex col [[1 2 3] [4 5 6]] 0 = [1 4]
 map lam r else at r i 0 m
fn transpose m:list list i64 -> list list i64
 ex transpose [[1 2 3] [4 5 6]] = [[1 4] [2 5] [3 6]]
 ex transpose empty list i64 = empty list i64
 if == len m 0 empty list i64 map lam i col m i range 0 len else at m 0 empty i64
```

### brackets
```aleph
fn bmatch o:str c:str -> bool
 ex bmatch "(" ")" = true
 ex bmatch "(" "]" = false
 if == o "(" == c ")" if == o "[" == c "]" if == o "{" == c "}" false
fn bpop st:list str c:str -> res list str str
 ex bpop ["("] ")" = ok empty str
 ex bpop empty str ")" = err "bad"
 let top else at st - len st 1 ""
 if == len st 0 err "bad" if bmatch top c ok slice st 0 - len st 1 err "bad"
fn bstep st:list str c:str -> res list str str
 ex bstep empty str "(" = ok ["("]
 ex bstep ["("] ")" = ok empty str
 ex bstep empty str ")" = err "bad"
 let isopen or == c "(" or == c "[" == c "{"
 let isclose or == c ")" or == c "]" == c "}"
 if isopen ok push st c if isclose bpop st c ok st
fn brackets s:str -> bool
 ex brackets "([]{x})" = true
 ex brackets "(]" = false
 ex brackets "([)]" = false
 let r fold @bstep ok empty str split s ""
 match r ok st == len st 0 err e false
```

### wrap
```aleph
type wstate lines:list str cur:str
fn wstep a:wstate word:str w:i64 -> wstate
 ex wstep wstate empty str "" "a" 3 = wstate empty str "a"
 let c len .cur a
 if == c 0 wstate .lines a word if <= + c + 1 len word w wstate .lines a cat cat .cur a " " word wstate push .lines a .cur a word
fn wrap s:str w:i64 -> list str
 ex wrap "a bb ccc d" 4 = ["a bb" "ccc" "d"]
 ex wrap "" 3 = empty str
 let ws fields s
 let st fold lam a x wstep a x w wstate empty str "" ws
 if == len .cur st 0 .lines st push .lines st .cur st
```

### bank
```aleph
fn bcmd bal:i64 c:str -> res i64 str
 ex bcmd 10 "dep 5" = ok 15
 ex bcmd 10 "wd 5" = ok 5
 ex bcmd 0 "wd 1" = err "overdraft"
 ex bcmd 0 "x" = err "bad"
 let fs fields c
 let op else at fs 0 ""
 let raw else at fs 1 ""
 let n try okor int raw "bad"
 if != len fs 2 err "bad" if < n 0 err "bad" if == op "dep" ok + bal n if == op "wd" if < - bal n 0 err "overdraft" ok - bal n err "bad"
fn bankstep acc:res i64 str c:str -> res i64 str
 ex bankstep ok 0 "dep 1" = ok 1
 let bal try acc
 bcmd bal c
fn bank cs:list str -> res i64 str
 ex bank ["dep 10" "wd 3"] = ok 7
 ex bank ["wd 1"] = err "overdraft"
 fold @bankstep ok 0 cs
```

### jsonsum
```aleph
fn jsonsum s:str -> res i64 str
 ex jsonsum "[1, 2, 3]" = ok 6
 ex isok jsonsum "[1, \"a\"]" = false
 let j try jparse s
 let xs try okor jlist j "bad"
 let ns try mapr lam x okor jint x "bad" xs
 ok sum ns
```

### median
```aleph
fn median xs:list i64 -> opt f64
 ex median [3 1 2] = some 2.0
 ex median empty i64 = none f64
 ex median [1 2 3 4] = some 2.5
 let ys sort xs
 let n len ys
 if == n 0 none f64 if == % n 2 1 some tof else at ys / n 2 0 some / + tof else at ys - / n 2 1 0 tof else at ys / n 2 0 2.0
```

### primes
```aleph
fn hasdiv k:i64 d:i64 -> bool
 ex hasdiv 9 2 = true
 ex hasdiv 7 2 = false
 if > * d d k false if == % k d 0 true hasdiv k + d 1
fn isprime k:i64 -> bool
 ex isprime 7 = true
 ex isprime 1 = false
 if < k 2 false not hasdiv k 2
fn primes n:i64 -> list i64
 ex primes 10 = [2 3 5 7]
 ex primes 1 = empty i64
 if < n 2 empty i64 filter @isprime range 2 + n 1
```

### titlecase
```aleph
fn cap w:str -> str
 ex cap "wORLD" = "World"
 let cs split w ""
 let h upper else at cs 0 ""
 let t lower join slice cs 1 len cs ""
 cat h t
fn titlecase s:str -> str
 ex titlecase "hello  wORLD" = "Hello World"
 ex titlecase "" = ""
 join map @cap fields s " "
```

### gcdlist
```aleph
fn gcd2 a:i64 b:i64 -> i64
 ex gcd2 12 18 = 6
 ex gcd2 0 5 = 5
 if == b 0 a gcd2 b % a b
fn gcdlist xs:list i64 -> i64
 ex gcdlist [12 18 30] = 6
 ex gcdlist empty i64 = 0
 fold @gcd2 0 xs
```

### kvdict
```aleph
fn kvpair d:dict str i64 p:str -> res dict str i64 str
 ex isok kvpair dict str i64 "a=1" = true
 ex isok kvpair dict str i64 "a" = false
 let ps split p "="
 let k else at ps 0 ""
 let v else at ps 1 ""
 let n try okor int v "bad"
 if != len ps 2 err "bad" if == len k 0 err "bad" ok put d k n
fn kvstep acc:res dict str i64 str p:str -> res dict str i64 str
 ex isok kvstep ok dict str i64 "a=2" = true
 let d try acc
 kvpair d p
fn kvdict s:str -> res dict str i64 str
 ex kvdict "a=1,b=2" = ok put put dict str i64 "a" 1 "b" 2
 ex isok kvdict "a" = false
 if == len s 0 ok dict str i64 fold @kvstep ok dict str i64 split s ","
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
fn dfs g:dict str list str seen:dict str bool n:str -> dict str bool
 ex size dfs put dict str list str "a" ["b"] dict str bool "a" = 2
 if has get seen n seen fold lam acc x dfs g acc x put seen n true else get g n empty str
fn reachable g:dict str list str s:str -> i64
 ex reachable put put dict str list str "a" ["b"] "b" ["a" "c"] "a" = 3
 ex reachable dict str list str "x" = 1
 size dfs g dict str bool s
```

### secondmax
```aleph
fn secondmax xs:list i64 -> opt i64
 ex secondmax [3 1 3 2] = some 2
 ex secondmax [5 5] = none i64
 let ys sort xs
 let n len ys
 let m else at ys - n 1 0
 let lower filter lam x < x m ys
 if < n 2 none i64 if == len lower 0 none i64 some else at sort lower - len lower 1 0
```
