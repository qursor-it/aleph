### wordcount
```aleph
fn wordcount s:str -> dict str i64
 ex wordcount "a b a" = put put dict str i64 "a" 2 "b" 1
 fold lam d w put d w + 1 else get d w 0 dict str i64 filter lam w > count w 0 split s " "
```

### topkey
```aleph
fn topkey d:dict str i64 -> opt str
 ex topkey dict str i64 = none str
 at sortby lam k neg else get d k 0 keys d 0
```

### safediv
```aleph
fn safediv a:i64 b:i64 -> res i64 str
 ex safediv 1 0 = err "div by zero"
 if == b 0 err "div by zero" ok / a b
```

### sumints
```aleph
fn num t:str -> res i64 str
 ex num "x" = err "x"
 if isok int t ok else int t 0 err t
fn sumints s:str -> res i64 str
 ex sumints "1 2 3" = ok 6
 let ns try mapr @num filter lam w > count w 0 split s " "
 ok sum ns
```

### firstneg
```aleph
fn firstneg xs:list i64 -> opt i64
 ex firstneg [3 -1 -2] = some 1
 at filter lam i < else at xs i 0 0 range 0 len xs 0
```

### area
```aleph
type shape | circle r:f64 | rect w:f64 h:f64
fn area s:shape -> f64
 ex area rect 2.0 3.0 = 6.0
 match s circle r * 3.14159 * r r rect w h * w h
```

### oldest
```aleph
type person name:str age:i64
fn oldest ps:list person -> opt str
 ex oldest empty person = none str
 let best fold lam a p max a .age p -1 ps
 at map lam p .name p filter lam p == .age p best ps 0
```

### chunks
```aleph
fn chunks xs:list i64 n:i64 -> list list i64
 ex chunks [1 2 3 4 5] 2 = [[1 2] [3 4] [5]]
 map lam i slice xs * i n + * i n n range 0 / + len xs - n 1 n
```

### pairsum
```aleph
fn pairsum xs:list i64 t:i64 -> bool
 ex pairsum [1 4 6] 10 = true
 < 0 len filter lam i < 0 len filter lam j and != i j == t + else at xs i 0 else at xs j 0 range 0 len xs range 0 len xs
```

### csvrow
```aleph
fn csvrow line:str -> list str
 ex csvrow "a, b ,c" = ["a" "b" "c"]
 map @trim split line ","
```

### initials
```aleph
fn initials s:str -> str
 ex initials "ada lovelace" = "AL"
 upper join map lam w else at split w "" 0 "" split s " " ""
```

### cumsum
```aleph
fn cumsum xs:list i64 -> list i64
 ex cumsum [1 2 3] = [1 3 6]
 fold lam acc x push acc + x else at rev acc 0 0 empty i64 xs
```

### jname
```aleph
fn jname text:str -> res str str
 ex jname "[1]" = err "bad"
 let j else jparse text json nil
 let d else jdict j dict str json
 let v else get d "name" json nil
 if has jstr v ok else jstr v "" err "bad"
```

### evensq
```aleph
fn evensq xs:list i64 -> list i64
 ex evensq [1 2 3 4] = [4 16]
 map lam x * x x filter lam x == 0 % x 2 xs
```

### pad
```aleph
fn pad s:str w:i64 -> str
 ex pad "7" 3 = "007"
 cat repeat "0" - w count s s
```

### maxrun
```aleph
fn maxrun xs:list i64 -> i64
 ex maxrun [1 1 2 2 2 1] = 3
 fold @max 0 fold lam acc i push acc if and > i 0 == else at xs i 0 else at xs - i 1 0 + 1 else at rev acc 0 0 1 empty i64 range 0 len xs
```

### lookup
```aleph
fn lookup kvs:list str k:str d:str -> str
 ex lookup ["a=1"] "z" "?" = "?"
 else at map lam kv else at split kv "=" 1 "" filter lam kv == k else at split kv "=" 0 "" kvs 0 d
```

### countif
```aleph
fn countif xs:list str p:str -> i64
 ex countif ["ab" "ac" "b"] "a" = 2
 len filter lam x starts x p xs
```

### fizz
```aleph
fn fizz n:i64 -> list str
 ex fizz 5 = ["1" "2" "Fizz" "4" "Buzz"]
 map lam i if == 0 % i 15 "FizzBuzz" if == 0 % i 3 "Fizz" if == 0 % i 5 "Buzz" show i range 1 + n 1
```

### avgint
```aleph
fn avgint xs:list i64 -> opt i64
 ex avgint empty i64 = none i64
 if == 0 len xs none i64 some / sum xs len xs
```
