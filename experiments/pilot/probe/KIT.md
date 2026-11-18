# Probe kit: does the model write Aleph directly or translate from Python?

Use a model that shows its reasoning (for example DeepSeek with DeepThink). Start a **new chat for each problem**, so earlier answers do not help.

1. Paste the whole of [`llms.txt`](../../../llms.txt) as the first part of the message.
2. Under it, paste the instruction and one problem from below.
3. Save the answer: the full reasoning and the code. Put them in a pilot session (`python3 experiments/pilot/tools/pilot.py new probe-NN`), or send them back as they are.

## Instruction (paste under llms.txt)

```text
Complete the Aleph function below so that it does what its doc line says. Write the whole file: the function exactly as given (header, doc and ex lines), any ex lines of your own, the body, and any helper functions you need (each with at least one ex line). Do not write main. Reply with only the code, in one code block marked aleph.
```

## 01 · HumanEval_13_greatest_common_divisor

```aleph
fn greatestcommondivisor a:i64 b:i64 -> i64
 doc "Return a greatest common divisor of two integers a and b"
 ex greatestcommondivisor 3 5 = 1
 ex greatestcommondivisor 25 15 = 5
```

## 02 · HumanEval_23_strlen

```aleph
fn strlen string:str -> i64
 doc "Return length of given string"
 ex strlen "" = 0
 ex strlen "abc" = 3
```

## 03 · HumanEval_30_get_positive

```aleph
fn getpositive l:list i64 -> list i64
 doc "Return only positive numbers in the list."
 ex getpositive [-1 2 -4 5 6] = [2 5 6]
 ex getpositive [5 3 -5 2 -3 3 9 0 123 1 -10] = [5 3 2 3 9 123 1]
```

## 04 · HumanEval_34_unique

```aleph
fn unique l:list i64 -> list i64
 doc "Return sorted unique elements in a list"
 ex unique [5 3 5 2 3 3 9 0 123] = [0 2 3 5 9 123]
```

## 05 · HumanEval_35_max_element

```aleph
fn maxelement l:list i64 -> i64
 doc "Return maximum element in the list."
 ex maxelement [1 2 3] = 3
 ex maxelement [5 3 -5 2 -3 3 9 0 123 1 -10] = 123
```

## 06 · HumanEval_48_is_palindrome

```aleph
fn ispalindrome text:str -> bool
 doc "Checks if given string is a palindrome"
 ex ispalindrome "" = true
 ex ispalindrome "aba" = true
 ex ispalindrome "aaaaa" = true
 ex ispalindrome "zbcd" = false
```

## 07 · HumanEval_55_fib

```aleph
fn fib n:i64 -> i64
 doc "Return n-th Fibonacci number."
 ex fib 10 = 55
 ex fib 1 = 1
 ex fib 8 = 21
```

## 08 · HumanEval_66_digitSum

```aleph
fn digitsum s:str -> i64
 doc "Task\nWrite a function that takes a string as input and returns the sum of the upper characters only'\nASCII codes.\n\nExamples:"
 ex digitsum "" = 0
 ex digitsum "abAB" = 131
 ex digitsum "abcCd" = 67
 ex digitsum "helloE" = 69
 ex digitsum "woArBld" = 131
 ex digitsum "aAaaaXa" = 153
```

## 09 · HumanEval_150_x_or_y

```aleph
fn xory n:i64 x:i64 y:i64 -> i64
 doc "A simple program which should return the value of x if n is\na prime number and should return the value of y otherwise.\n\nExamples:"
 ex xory 7 34 12 = 34
 ex xory 15 8 5 = 5
```

## 10 · HumanEval_159_eat

```aleph
fn eat number:i64 need:i64 remaining:i64 -> list i64
 doc "You're a hungry rabbit, and you already have eaten a certain number of carrots,\nbut now you need to eat more carrots to complete the day's meals.\nyou should return an array of [ total number of eaten carrots after your meals,\n                                the number of carrots left after your meals ]\nif there are not enough remaining carrots, you will eat all remaining carrots, but will still be hungry.\n\nExample:\n\nVariables:\n@number : integer\n    the number of carrots that you have eaten.\n@need : integer\n    the number of carrots that you need to eat.\n@remaining : integer\n    the number of remaining carrots thet exist in stock\n\nConstrain:\n* 0 <= number <= 1000\n* 0 <= need <= 1000\n* 0 <= remaining <= 1000\n\nHave fun :)"
 ex eat 5 6 10 = [11 4]
 ex eat 4 8 9 = [12 1]
 ex eat 1 10 10 = [11 0]
 ex eat 2 11 5 = [7 0]
```
