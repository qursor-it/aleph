### wordcount
```aleph
fn wordcount s:str -> dict str i64
 ex wordcount "a b a" = put put dict str i64 "a" 2 "b" 1
 ex wordcount "" = dict str i64
 fold lam acc w if == w "" acc put acc w + else get acc w 0 1 dict str i64 split s " "
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
 let ns try mapr lam t okor int t t split s " "
 ok sum ns
```

### firstneg
```aleph
fn firstneg xs:list i64 -> opt i64
 ex firstneg [3 -1 -2] = some 1
 ex firstneg [1 2] = none i64
 at filter lam i < else at xs i 0 0 range 0 len xs 0
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
 let sp sortby lam m neg .age m ps
 at map lam q .name q sp 0
```

### chunks
```aleph
fn chunks xs:list i64 n:i64 -> list list i64
 ex chunks [1 2 3 4 5] 2 = [[1 2] [3 4] [5]]
 ex chunks empty i64 3 = empty list i64
 map lam i slice xs * i n + * i n n range 0 / + len xs - n 1 n
```

### pairsum
```aleph
fn pairsum xs:list i64 t:i64 -> bool
 ex pairsum [1 4 6] 10 = true
 ex pairsum [5] 10 = false
 > len filter lam b b map lam i has index cat slice xs 0 i slice xs + i 1 len xs - t else at xs i 0 range 0 len xs 0
```

### csvrow
```aleph
fn csvrow line:str -> list str
 ex csvrow "a, b ,c" = ["a" "b" "c"]
 ex csvrow "x" = ["x"]
 map @trim split line ","
```

### initials
```aleph
fn initials s:str -> str
 ex initials "ada lovelace" = "AL"
 ex initials "" = ""
 join map lam w upper else at split w "" 0 "" filter lam x != x "" split s " " ""
```

### cumsum
```aleph
fn cumsum xs:list i64 -> list i64
 ex cumsum [1 2 3] = [1 3 6]
 ex cumsum empty i64 = empty i64
 map lam i sum slice xs 0 + i 1 range 0 len xs
```

### jname
```aleph
fn jname text:str -> res str str
 ex jname "{\"name\":\"ann\"}" = ok "ann"
 ex jname "[1]" = err "bad"
 let j try okor jparse text "bad"
 let d try okor jdict j "bad"
 let v try okor get d "name" "bad"
 ok try okor jstr v "bad"
```

### evensq
```aleph
fn evensq xs:list i64 -> list i64
 ex evensq [1 2 3 4] = [4 16]
 ex evensq [1 3] = empty i64
 map lam x * x x filter lam y == % y 2 0 xs
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
 let fs fold lam st x if == else at st 1 0 0 [x 1 max else at st 2 0 1] if == x else at st 0 0 [x + else at st 1 0 1 max else at st 2 0 + else at st 1 0 1] [x 1 else at st 2 0] [0 0 0] xs
 else at fs 2 0
```

### lookup
```aleph
fn lookup kvs:list str k:str d:str -> str
 ex lookup ["a=1" "b=2"] "b" "?" = "2"
 ex lookup ["a=1"] "z" "?" = "?"
 else fold lam acc kv if has acc acc if == else at split kv "=" 0 "" k some else at split kv "=" 1 "" acc none str kvs d
```

### countif
```aleph
fn countif xs:list str p:str -> i64
 ex countif ["ab" "ac" "b"] "a" = 2
 ex countif empty str "a" = 0
 len filter lam x starts x p xs
```

### fizz
```aleph
fn fizz n:i64 -> list str
 ex fizz 5 = ["1" "2" "Fizz" "4" "Buzz"]
 ex fizz 0 = empty str
 map lam i if == % i 15 0 "FizzBuzz" if == % i 3 0 "Fizz" if == % i 5 0 "Buzz" show i range 1 + n 1
```

### avgint
```aleph
fn avgint xs:list i64 -> opt i64
 ex avgint [1 2 4] = some 2
 ex avgint empty i64 = none i64
 if == len xs 0 none i64 some / sum xs len xs
```
