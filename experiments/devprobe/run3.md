### wordcount
```aleph
fn wc1 d:dict str i64 w:str -> dict str i64
 ex wc1 dict str i64 "a" = put dict str i64 "a" 1
 if == w "" d
 put d w + else get d w 0 1
fn wordcount s:str -> dict str i64
 ex wordcount "a b a" = put put dict str i64 "a" 2 "b" 1
 ex wordcount "" = dict str i64
 fold @wc1 dict str i64 split s " "
```

### topkey
```aleph
fn topkey d:dict str i64 -> opt str
 ex topkey put put dict str i64 "a" 1 "b" 3 = some "b"
 ex topkey dict str i64 = none str
 at sortby lam k neg else get d k 0 keys d 0
```

### safediv
```aleph
fn safediv a:i64 b:i64 -> res i64 str
 ex safediv 7 2 = ok 3
 ex safediv 1 0 = err "div by zero"
 if == b 0 err "div by zero" ok / a b
```

### sumints
```aleph
fn sumints s:str -> res i64 str
 ex sumints "1 2 3" = ok 6
 ex sumints "1 x 3" = err "x"
 if == s "" ok 0
 let ns try mapr lam t if isok int t ok else int t 0 err t split s " "
 ok sum ns
```

### firstneg
```aleph
fn firstneg xs:list i64 -> opt i64
 ex firstneg [3 -1 -2] = some 1
 ex firstneg [1 2] = none i64
 let is filter lam i < else at xs i 0 0 range 0 len xs
 if == len is 0 none i64 at is 0
```

### area
```aleph
type shape | circle r:f64 | rect w:f64 h:f64
fn area s:shape -> f64
 ex area rect 2.0 3.0 = 6.0
 ex area circle 1.0 = 3.14159
 match s circle r * 3.14159 * r r rect w h * w h
```

### oldest
```aleph
type person name:str age:i64
fn oldest ps:list person -> opt str
 ex oldest [person "a" 30 person "b" 40] = some "b"
 ex oldest empty person = none str
 let q sortby lam p neg .age p ps
 if == len q 0 none str some .name else at q 0 person "" 0
```

### chunks
```aleph
fn chunks xs:list i64 n:i64 -> list list i64
 ex chunks [1 2 3 4 5] 2 = [[1 2] [3 4] [5]]
 ex chunks empty i64 3 = empty list i64
 if == len xs 0 empty list i64
 push chunks slice xs n len xs n slice xs 0 n
```

### pairsum
```aleph
fn ps2 xs:list i64 t:i64 i:i64 j:i64 -> bool
 ex ps2 [1 4] 5 0 1 = true
 if == j len xs false
 if == + else at xs i 0 else at xs j 0 t true ps2 xs t i + j 1
fn ps1 xs:list i64 t:i64 i:i64 -> bool
 ex ps1 [1 4 6] 10 0 = true
 if == i len xs false
 if ps2 xs t i + i 1 true ps1 xs t + i 1
fn pairsum xs:list i64 t:i64 -> bool
 ex pairsum [1 4 6] 10 = true
 ex pairsum [5] 10 = false
 ps1 xs t 0
```

### csvrow
```aleph
fn csvrow line:str -> list str
 ex csvrow "a, b ,c" = ["a" "b" "c"]
 ex csvrow "x" = ["x"]
 map lam f trim f split line ","
```

### initials
```aleph
fn initials s:str -> str
 ex initials "ada lovelace" = "AL"
 ex initials "" = ""
 join map lam w upper else at split w "" 0 "" split s " " ""
```

### cumsum
```aleph
fn cs1 xs:list i64 acc:i64 -> list i64
 ex cs1 [1 2 3] 0 = [1 3 6]
 if == len xs 0 empty i64
 let a + acc else at xs 0 0
 push cs1 slice xs 1 len xs a a
fn cumsum xs:list i64 -> list i64
 ex cumsum [1 2 3] = [1 3 6]
 ex cumsum empty i64 = empty i64
 cs1 xs 0
```

### jname
```aleph
fn jname text:str -> res str str
 ex jname "{\"name\":\"ann\"}" = ok "ann"
 ex jname "[1]" = err "bad"
 let j try jparse text
 if not has jdict j err "bad"
 let d else jdict j dict str json
 let n else get d "name" json dict str json
 if not has jstr n err "bad"
 ok else jstr n ""
```

### evensq
```aleph
fn evensq xs:list i64 -> list i64
 ex evensq [1 2 3 4] = [4 16]
 ex evensq [1 3] = empty i64
 map lam v * v v filter lam x == % x 2 0 xs
```

### pad
```aleph
fn pad s:str w:i64 -> str
 ex pad "7" 3 = "007"
 ex pad "1234" 2 = "1234"
 if <= w count s s cat repeat "0" - w count s s
```

### maxrun
```aleph
fn mr1 xs:list i64 prev:opt i64 cur:i64 best:i64 -> i64
 ex mr1 [1 1] none i64 0 0 = 2
 if == len xs 0 max best cur
 let x else at xs 0 0
 let c if and has prev == x else prev 0 + cur 1 1
 mr1 slice xs 1 len xs some x c max best c
fn maxrun xs:list i64 -> i64
 ex maxrun [1 1 2 2 2 1] = 3
 ex maxrun empty i64 = 0
 mr1 xs none i64 0 0
```

### lookup
```aleph
fn lk1 kvs:list str k:str d:str -> str
 ex lk1 ["a=1"] "a" "?" = "1"
 if == len kvs 0 d
 let p split else at kvs 0 "" "="
 if == else at p 0 "" k join slice p 1 len p "=" lk1 slice kvs 1 len kvs k d
fn lookup kvs:list str k:str d:str -> str
 ex lookup ["a=1" "b=2"] "b" "?" = "2"
 ex lookup ["a=1"] "z" "?" = "?"
 lk1 kvs k d
```

### countif
```aleph
fn countif xs:list str p:str -> i64
 ex countif ["ab" "ac" "b"] "a" = 2
 ex countif empty str "a" = 0
 len filter lam s starts s p xs
```

### fizz
```aleph
fn fz1 x:i64 -> str
 ex fz1 3 = "Fizz"
 ex fz1 15 = "FizzBuzz"
 if and == % x 3 0 == % x 5 0 "FizzBuzz" if == % x 3 0 "Fizz" if == % x 5 0 "Buzz" show x
fn fizz n:i64 -> list str
 ex fizz 5 = ["1" "2" "Fizz" "4" "Buzz"]
 ex fizz 0 = empty str
 map @fz1 range 1 + n 1
```

### avgint
```aleph
fn avgint xs:list i64 -> opt i64
 ex avgint [1 2 4] = some 2
 ex avgint empty i64 = none i64
 if == len xs 0 none i64
 some / sum xs len xs
```
