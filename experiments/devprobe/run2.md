### wordcount
```aleph
fn wordcount s:str -> dict str i64
 ex wordcount "a b a" = put put dict str i64 "a" 2 "b" 1
 ex wordcount "" = dict str i64
 fold @wcstep dict str i64 split s " "
fn wcstep d:dict str i64 w:str -> dict str i64
 ex wcstep dict str i64 "" = dict str i64
 ex wcstep dict str i64 "a" = put dict str i64 "a" 1
 if == w "" d put d w + else get d w 0 1
```

### topkey
```aleph
fn topkey d:dict str i64 -> opt str
 ex topkey put put dict str i64 "a" 1 "b" 3 = some "b"
 ex topkey dict str i64 = none str
 ex topkey put put dict str i64 "a" 3 "b" 3 = some "a"
 if == size d 0 none str at keys d else index vals d fold @max else at vals d 0 0 vals d 0
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
 fold @sip ok 0 split s " "
fn sip acc:i64 t:str -> res i64 str
 ex sip 1 "2" = ok 3
 ex sip 1 "x" = err "x"
 if isok int t ok + acc else int t 0 err t
```

### firstneg
```aleph
fn firstneg xs:list i64 -> opt i64
 ex firstneg [3 -1 -2] = some 1
 ex firstneg [1 2] = none i64
 fnh xs 0
fn fnh xs:list i64 i:i64 -> opt i64
 ex fnh [3 -1] 0 = some 1
 ex fnh [1 2] 0 = none i64
 let x else at xs i 0
 if >= i len xs none i64 if < x 0 some i fnh xs + i 1
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
 let sp sortby lam p neg .age p ps
 if == len sp 0 none str some .name else at sp 0 person "" 0
```

### chunks
```aleph
fn chunks xs:list i64 n:i64 -> list list i64
 ex chunks [1 2 3 4 5] 2 = [[1 2] [3 4] [5]]
 ex chunks empty i64 3 = empty list i64
 ch xs n empty list i64
fn ch xs:list i64 n:i64 acc:list list i64 -> list list i64
 ex ch [1 2 3] 2 empty list i64 = [[1 2] [3]]
 ex ch empty i64 2 empty list i64 = empty list i64
 if == len xs 0 acc ch slice xs n len xs n push acc slice xs 0 n
```

### pairsum
```aleph
fn pairsum xs:list i64 t:i64 -> bool
 ex pairsum [1 4 6] 10 = true
 ex pairsum [5] 10 = false
 psi xs t 0
fn psi xs:list i64 t:i64 i:i64 -> bool
 ex psi [1 4 6] 10 0 = true
 ex psi [5] 10 0 = false
 let x else at xs i 0
 let j index xs - t x
 if >= i len xs false if and has j != else j 0 i true psi xs t + i 1
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
 join map @ini split s " " ""
fn ini w:str -> str
 ex ini "ada" = "A"
 ex ini "" = ""
 upper join slice split w "" 0 1 ""
```

### cumsum
```aleph
fn cumsum xs:list i64 -> list i64
 ex cumsum [1 2 3] = [1 3 6]
 ex cumsum empty i64 = empty i64
 fold @cs empty i64 xs
fn cs acc:list i64 x:i64 -> list i64
 ex cs [1] 2 = [1 3]
 ex cs empty i64 5 = [5]
 push acc + else at acc - len acc 1 0 x
```

### jname
```aleph
fn jname text:str -> res str str
 ex jname "{\"name\":\"ann\"}" = ok "ann"
 ex jname "[1]" = err "bad"
 ex jname "nope" = err "bad"
 ex jname "{\"name\":1}" = err "bad"
 let j else jparse text json nil
 let o else jdict j dict str json
 let s jstr else get o "name" json nil
 if has s ok else s "" err "bad"
```

### evensq
```aleph
fn evensq xs:list i64 -> list i64
 ex evensq [1 2 3 4] = [4 16]
 ex evensq [1 3] = empty i64
 map lam x * x x filter lam x == 0 % x 2 xs
```

### pad
```aleph
fn pad s:str w:i64 -> str
 ex pad "7" 3 = "007"
 ex pad "1234" 2 = "1234"
 if >= count s w s cat repeat "0" - w count s s
```

### maxrun
```aleph
fn maxrun xs:list i64 -> i64
 ex maxrun [1 1 2 2 2 1] = 3
 ex maxrun empty i64 = 0
 mr xs 0 0 0
fn mr xs:list i64 i:i64 cur:i64 best:i64 -> i64
 ex mr [1 1 2] 0 0 0 = 2
 ex mr empty i64 0 0 0 = 0
 let x else at xs i 0
 let p else at xs - i 1 0
 let c if and > i 0 == x p + cur 1 1
 if >= i len xs best mr xs + i 1 c max best c
```

### lookup
```aleph
fn lookup kvs:list str k:str d:str -> str
 ex lookup ["a=1" "b=2"] "b" "?" = "2"
 ex lookup ["a=1"] "z" "?" = "?"
 lkh kvs k d 0
fn lkh kvs:list str k:str d:str i:i64 -> str
 ex lkh ["a=1" "b=2"] "b" "?" 0 = "2"
 ex lkh ["a=1"] "z" "?" 0 = "?"
 let kv else at kvs i ""
 let p split kv "="
 if >= i len kvs d if and == len p 2 == else at p 0 "" k else at p 1 "" lkh kvs k d + i 1
```

### countif
```aleph
fn countif xs:list str p:str -> i64
 ex countif ["ab" "ac" "b"] "a" = 2
 ex countif empty str "a" = 0
 cih xs p 0
fn cih xs:list str p:str i:i64 -> i64
 ex cih ["ab" "ac" "b"] "a" 0 = 2
 ex cih empty str "a" 0 = 0
 let s else at xs i ""
 if >= i len xs 0 + if starts s p 1 0 cih xs p + i 1
```

### fizz
```aleph
fn fizz n:i64 -> list str
 ex fizz 5 = ["1" "2" "Fizz" "4" "Buzz"]
 ex fizz 0 = empty str
 map @fz range 1 + n 1
fn fz i:i64 -> str
 ex fz 3 = "Fizz"
 ex fz 15 = "FizzBuzz"
 ex fz 7 = "7"
 if == 0 % i 15 "FizzBuzz" if == 0 % i 3 "Fizz" if == 0 % i 5 "Buzz" show i
```

### avgint
```aleph
fn avgint xs:list i64 -> opt i64
 ex avgint [1 2 4] = some 2
 ex avgint empty i64 = none i64
 ex avgint [1 2] = some 1
 if == len xs 0 none i64 some / sum xs len xs
```
