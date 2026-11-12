# Benchmark and Orbis seed tasks (v0.1 feature set)

Each task: write an Aleph program with examples for every non-main function. Hidden tests compare stdout.

1. **fib** — print the 30th Fibonacci number.
2. **total** — print the sum of the integers given as command line arguments (`args`, `int`, `try`).
3. **evens** — print the even numbers from 0 to 20 joined by spaces.
4. **primes** — print all primes below 100, one per line.
5. **fizzbuzz** — print fizzbuzz from 1 to 30.
6. **palindrome** — for each argument print `yes` or `no` if it is a palindrome (ignore case).
7. **wordcount** — read `data/in.txt` and print the number of words (needs `use fs.read "data"`).
8. **linecount** — read `data/in.txt` and print the number of non-empty lines.
9. **maxmin** — print max and min of integer arguments; print an error to stderr and return err if none.
10. **points** — define `type point x:f64 y:f64`, compute the perimeter of a polygon given as a list of points.
11. **reverse** — print the words of each argument in reverse order.
12. **gcd** — gcd and lcm of two integer arguments.
13. **upper** — read `data/in.txt`, write its uppercase version to `out/up.txt` (needs fs.read and fs.write).
14. **histogram** — print a text histogram (`*` repeated) of the integer arguments.
15. **collatz** — print the Collatz sequence length for 1..20.
