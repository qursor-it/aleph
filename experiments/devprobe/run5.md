### wordcount
```aleph
fn wordcount s:str -> dict str i64
 ex wordcount "a b a" = put put dict str i64 "a" 2 "b" 1
 ex wordcount "" = dict str i64
 fold lam d w put d w + else get d w 0 1 dict str i64 filter lam v != v "" split s " "
```
### topkey
```aleph
fn topkey d:dict str i64 -> opt str
 ex topkey put put dict str i64 "a" 1 "b" 3 = some "b"
 ex topkey dict str i64 = none str
 let ks rev keys d
 let ss sortby lam k else get d k 0 ks
 at ss - len ss 1
```
### safediv
```aleph
fn safediv a:i64 b:i64 -> res i64 str
 ex safediv 7 2 = ok 3
 ex safediv 1 0 = err "div by zero"
 if == b 0 err "div by zero"
 ok / a b
```
### sumints
```aleph
fn sumints s:str -> res i64 str
 ex sumints "1 2 3" = ok 6
 ex sumints "1 x 3" = err "x"
 let ns try mapr @int split s " "
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
 if == 0 len ps none str
 let ss sortby lam p .age p rev ps
 some .name else at ss - len ss 1 person "" 0
```
### chunks
```aleph
fn chunks xs:list i64 n:i64 -> list list i64
 ex chunks [1 2 3 4 5] 2 = [[1 2] [3 4] [5]]
 ex chunks empty i64 3 = empty list i64
 if == 0 len xs empty list i64
 push chunks slice xs n len xs n slice xs 0 n
```
### pairsum
```aleph
fn pairsum xs:list i64 t:i64 -> bool
 ex pairsum [1 4 6] 10 = true
 ex pairsum [5] 10 = false
 fold lam acc i or acc haspair xs t i false range 0 len xs
fn haspair xs:list i64 t:i64 i:i64 -> bool
 ex haspair [1 4 6] 10 1 = true
 ex haspair [5] 10 0 = false
 fold lam acc j and != i j == t + else at xs i 0 else at xs j 0 false range 0 len xs
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
 join map lam w upper else at split w "" 0 "" filter lam v != v "" split s " " ""
```
### cumsum
```aleph
fn cumsum xs:list i64 -> list i64
 ex cumsum [1 2 3] = [1 3 6]
 ex cumsum empty i64 = empty i64
 fold lam acc x push acc + else at acc - len acc 1 0 x empty i64 xs
```
### jname
```aleph
fn jname text:str -> res str str
 ex jname "{\"name\":\"ann\"}" = ok "ann"
 ex jname "[1]" = err "bad"
 let p jparse text
 if not isok p err "bad"
 let j else p json nil
 if not has jdict j err "bad"
 let d else jdict j dict str json
 let o jstr else get d "name" json nil
 if not has o err "bad"
 ok else o ""
```
### evensq
```aleph
fn evensq xs:list i64 -> list i64
 ex evensq [1 2 3 4] = [4 16]
 ex evensq [1 3] = empty i64
 map lam x * x x filter lam y == 0 % y 2 xs
```
### pad
```aleph
fn pad s:str w:i64 -> str
 ex pad "7" 3 = "007"
 ex pad "1234" 2 = "1234"
 if <= w count s s
 cat repeat "0" - w count s s
```
### maxrun
```aleph
type runstate best:i64 cur:i64 prev:i64
fn maxrun xs:list i64 -> i64
 ex maxrun [1 1 2 2 2 1] = 3
 ex maxrun empty i64 = 0
 let r fold lam st x if == x .prev st runstate max .best st + .cur st 1 + .cur st 1 x runstate max .best st 1 1 x runstate 0 0 0 xs
 .best r
```
### lookup
```aleph
fn lookup kvs:list str k:str d:str -> str
 ex lookup ["a=1" "b=2"] "b" "?" = "2"
 ex lookup ["a=1"] "z" "?" = "?"
 else at map lam kv else at split kv "=" 1 "" filter lam kv2 == k else at split kv2 "=" 0 "" kvs 0 d
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
 map lam i if == 0 % i 15 "FizzBuzz" if == 0 % i 3 "Fizz" if == 0 % i 5 "Buzz" show i range 1 + n 1
```
### avgint
```aleph
fn avgint xs:list i64 -> opt i64
 ex avgint [1 2 4] = some 2
 ex avgint empty i64 = none i64
 if == 0 len xs none i64
 some / sum xs len xs
```
