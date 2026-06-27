# Fuga Language User Reference

> **Note**: This English version is a direct translation of the original Japanese version (`fuga-reference.md`). The Japanese version is the original and authoritative text. This English translation may contain inaccuracies.

Welcome to the fuga language!
The fuga language is a programming language with an extremely harsh memory management specification: "variables are consumed and vanish once they are used." At first glance, it may seem impossible to write a proper program, but by combining several mechanisms of the language (such as dynamic scoping and self-assignment) like a puzzle, you can write advanced processing and loops.

This document provides easy-to-understand explanations with sample code, ranging from basic usage of the fuga language to practical coding patterns.

---

## 1. Quick Start (Hello World)

First, let's look at a program that outputs "Hello, World!\n" to the standard output.

```fuga
(Store character codes in a global array)
array u8*D $hello = {48, 65, 6C, 6C, 6F, 2C, 20, 57, 6F, 72, 6C, 64, A}.

(Helper function to copy variables)
func i32 #copy_i = {
    $copy_i = $i. (Reference the caller's $i. External scope, so it is not consumed)
}.

(Main processing function)
func i32 #main = {
    var i32 $i = 0.
    try {
        loop {
            (Copy counter $i twice without consuming it. Variables declared in the loop will automatically vanish at the end of the loop)
            var i32 $i_for_check = #copy_i.
            var i32 $i_for_array = #copy_i.

            (Exit the loop when the counter reaches array length D (13). $i_for_check is consumed here)
            throw 1 @ $i_for_check = D.

            (Calculate the next index. $i is consumed here)
            var i32 $next_i = $i + 1.

            (Read the array with the copied index. $i_for_array is consumed here)
            var u8 $char = $hello[$i_for_array].

            (Output a single character. $char is consumed here)
            var i32 $unused = #_putc($char).

            (Redefine $i for the next iteration. $next_i is consumed here)
            $i = $next_i.
        }.
    } catch 1 {
        (Normal exit)
    }.
}.

var i32 $exit_code = #main.
```

---

## 2. Basic Syntax and Data Representation

### 2.1. End of Statement
All statements end with a period (`.`). Forgetting the period will result in a syntax error.

### 2.2. Comments
The parts enclosed in parentheses `( )` become comments and are ignored during execution. They can also be inserted in the middle of a statement.
```fuga
var i32 $a = 5. (This is a comment)
var i32 $b = (Insert in the middle) 10.
```

### 2.3. How to write constants (Hexadecimal)
All numbers are written in **hexadecimal**. We do not use prefixes like `0x` or `h` found in general programming languages. Also, only uppercase letters (`A`-`F`) can be used for alphabets.
* `A` is 10 in decimal
* `10` is 16 in decimal
* `-1A` is -26 in decimal

---

## 3. Variables and the "Consumption" Rule

### 3.1. Variable Declaration and Omitting Initial Values
Variables must be defined by specifying their type before using them. In the fuga language, **it is possible to omit the initial value specification** when declaring a variable. A variable with an omitted initial value is automatically initialized with **`0`**.

```fuga
var i32 $count = 5. (Defined with initial value 5)
var u8 $flag.        (Initial value omitted, automatically initialized to 0)
```

### 3.2. Basic Rule of Consumption
The biggest hurdle of the fuga language is the rule that **"a variable whose value has been read (evaluated) is consumed (invalidated) at that moment"**.

```fuga
var i32 $x = 5.
var i32 $y = $x. (The value of $x is read on this line, so $x is consumed)
var i32 $z = $x. (Error! $x is already invalid)
```

We will introduce 3 techniques to overcome this specification and handle variables safely.

### Technique 1: "Value Copying" using Dynamic Scope
The fuga language adopts **dynamic scope** (a specification that searches for variables by tracing back to the caller of the function). Furthermore, there is a special rule that **"when a variable in an external scope of the function is referenced, it becomes read-only, and referencing it does not consume it"**.

Using this, you can create a function that copies the value of a variable.

```fuga
(Function to copy the caller's variable $a without consuming it)
func i32 #copy_a = {
    $copy_a = $a. (References $a in the external scope, so it is not consumed)
}.

func i32 #example = {
    var i32 $a = A.
    var i32 $b = #copy_a. (This assigns A to $b. $a is not consumed!)
    var i32 $c = $a.       ($a is consumed here)
}.
```

### Technique 2: "Variable Update" by Self-assignment
Exceptionally, when the right side of an assignment expression contains exactly the same single variable as the left side (self-assignment), **the variable is not consumed but redefined with a new value**.

```fuga
var i32 $count = 0.
$count = $count + 1. (Valid: $count is not consumed, updated to 1)
```
*Note: An expression containing the same variable multiple times on the right side (e.g., `$x = $x + $x.`) will result in an error because the variable is consumed during the evaluation of the expression.

### Technique 3: "Variable Redefinition" using `var`
If you want to reuse a variable that has once been consumed with the same name, you cannot simply assign to it; you must redefine it using `var` again.

```fuga
var i32 $x = 1.
var i32 $y = $x. (Here $x is consumed)
var i32 $x = 2.  (Valid since it is a redefinition using var)
```

### 3.4. Implicit Type Conversion and Compiler Warnings
There is no cast syntax (such as `as` or casting with parentheses) to forcefully convert types in the fuga language. When assigning or passing arguments between different data types, the types are all **implicitly (automatically)** converted.

However, if there is a possibility that the value may change or the interpretation of the sign may go wrong due to the conversion, a **Warning** is output at compile time.

#### 1. Cases without warnings (Safe widening)
If the original data range completely fits within the destination data range, it is converted safely, and no warning is issued.
```fuga
var u8 $a = 5.
var u16 $b = $a. (No warning: Automatic widening from 8-bit unsigned to 16-bit unsigned)
```

#### 2. Cases with warnings (Sign mismatch / Narrowing conversion)
*   **Sign mismatch:** When converting between signed and unsigned types, the value is reinterpreted while keeping the bit representation intact.
    ```fuga
    var i8 $c = -1. (FF in hexadecimal)
    var u8 $d = $c. (Warning: Sign mismatch. Stored as 255 in $d)
    ```
*   **Bit width narrowing:** When converting from a larger type to a smaller type, the overflowing upper bits are truncated.
    ```fuga
    var u16 $e = 101. (257 in decimal. Bit representation is 0101)
    var u8 $f = $e.   (Warning: Narrowing conversion. Upper bits are truncated, and 1 is placed in $f)
    ```

---

## 4. How to use Arrays

An array is a mechanism for managing data of the same type side by side.

### 4.1. Array Definition
```fuga
array i32*3 $arr = {1, 2, 3}. (Define an array of 3 elements with initial values)
```

You can also specify a variable for the number of elements, but in that case, the variable is consumed, and you cannot specify initial values (all elements are initialized to 0).
```fuga
var i32 $size = 5.
array i32*$size $my_array. (Here $size is consumed)
```

### 4.2. Element Consumption and No Re-assignment Rule
When you read an element of an array (e.g., `$arr[1]`), **only that element is consumed**. Other elements are not affected.
Also, **you cannot re-assign (write a value) to an element slot that has once been consumed.**

```fuga
array i32*3 $arr = {A, B, C}.
var i32 $val = $arr[1]. (Here $arr[1] is consumed)
$arr[1] = 5.            (Error! Re-assignment to a consumed slot is impossible)
var i32 $val2 = $arr[2]. (Valid! $arr[2] has not been consumed)
```

### 4.3. Read-only Reference to External Arrays
When you reference an element of an array defined outside the function (e.g., globally) from within the function, **the element is not consumed.**

```fuga
array i32*3 $global_arr = {1, 2, 3}.

func i32 #read_global = {
    var i32 $val = $global_arr[0]. (Since it's an external array, $global_arr[0] is not consumed!)
}.
```

---

## 5. Operators and Calculations

In the fuga language, except for conditional expressions, **all operations must be accompanied by an assignment.** An independent calculation expression (e.g., `$a + 1.`) will result in an error.

Also, **the multiplication `*` and division `/` operators are not supported as language specifications.** To find a product or quotient, you must implement an auxiliary function (user-defined function) combining bit shifts or addition/subtraction loops.

### 5.1. Operator List (in order of precedence)

The operators available in the fuga language are as follows. Those higher in the table have higher priority and are evaluated first.

| Precedence | Operator | Classification | Associativity | Description | Example |
| :---: | :---: | :--- | :---: | :--- | :--- |
| 1 | `~` | Postfix Unary | None | Returns the bit-inverted value of the specified variable. The target variable is consumed. | `$a = $b~.` |
| 1 | `?` | Postfix Unary | None | Variable existence check. Returns 1 if valid (not consumed), 0 if invalid (consumed). **The target variable is not consumed.** | `$a = $b?.` |
| 2 | `<<`<br>`>>` | Binary | Left | Bitwise left shift and right shift. Signed types perform an arithmetic shift, Unsigned types perform a logical shift. | `$a = $b << 2.` |
| 3 | `&` | Binary | Left | Bitwise logical AND. | `$a = $b & 2.` |
| 4 | `\|` | Binary | Left | Bitwise logical OR. | `$a = $b \| 2.` |
| 5 | `+`<br>`-` | Binary | Left | Arithmetic addition and subtraction. | `$a = $b + 1.` |
| 6 | `<`<br>`>`<br>`=` | Binary(Comparison)| None | Comparison (less than, greater than, equal to). Returns 1 if true, 0 if false. Since there is no associativity, they cannot be chained. | `$a = $b > 3.` |
| 7 | `=` | Assignment | Right | Stores the evaluation result of the right side in the variable on the left side. | `$a = 1.` |

### 5.2. Distinguishing between Equality Comparison and Assignment

In the fuga language, both equality comparison and assignment use the same `=` symbol. When multiple `=` are written in one expression, **only the leftmost `=` functions as an assignment operator**, and all other `=` are treated as comparison operators.

```fuga
var i32 $a = 0.
var i32 $b = 3.
$a = $b = 3.
(Compares if $b is equal to 3, and assigns the result "1" to $a. At this time, $b is evaluated and thus consumed)
```

---

## 6. Conditional Branching and Iteration

### 6.1. Conditional Branching (Pattern using throw)
There is no `if` statement in the fuga language. To perform conditional branching, combine `try-catch` and `throw` and write it as exception handling.

```fuga
var i32 $a = 5.
try {
    ( Throw exception 1 if $a is greater than 0. $a is consumed )
    throw 1 @ $a > 0.
    ( This is executed only when $a is 0 or less )
    var i32 $result = 0.
} catch 1 {
    ( Processing when $a was greater than 0 )
    var i32 $result = 1.
}.
```

### 6.2. Iterative Processing (loop)
`loop` is an infinite loop. To break out of the loop, execute `loop` inside a `try`, and when the termination condition is met, jump to the `catch` outside the loop with `throw`.

```fuga
func i32 #copy_i = {
    $copy_i = $i.
}.

func i32 #main = {
    var i32 $i = 0.
    try {
        loop {
            var i32 $i_check = #copy_i.
            throw 9 @ $i_check = 5. ( When $i_check becomes 5, throw exception 9 and break out of the loop )
            var i32 $next = $i + 1. ( Calculate the incremented value )
            $i = $next.             ( Revive $i with the incremented value )
        }.
    } catch 9 {
        ( Processing after breaking out of the loop )
    }.
}.
```

---

## 7. How to Create and Call Functions

### 7.1. Function Definition and Return Value
The function name starts with `#`. The return value is set by assigning it to a variable where the function name prefix `#` is changed to `$`, and it is automatically returned when the function ends.

```fuga
func i32 #double(i32 $val) = {
    $double = $val << 1. (Set twice the value of $val to the return value variable $double)
}.
```

### 7.2. Notes on Argument Passing
When you pass a variable as an actual argument to a function, **that variable is immediately consumed in the caller's scope**.

```fuga
var i32 $my_num = 5.
var i32 $res = #double($my_num). (Here $my_num is consumed!)
var i32 $err = $my_num.          (Error! $my_num cannot be used)
```

---

## 8. Sample Programs

### 8.1. Multiplication of Numbers (Simulation by Addition)
Since there is no multiplication `*` operator, perform multiplication (`$a` × `$b`) using a loop and addition.

```fuga
(Helper functions for copying variables)
func i32 #copy_a = { $copy_a = $a. }.
func i32 #copy_b = { $copy_b = $b. }.

func i32 #multiply(i32 $a, i32 $b) = {
    var i32 $result = 0.
    try {
        loop {
            ( Copy $b and check the termination condition )
            var i32 $b_check = #copy_b.
            throw 1 @ $b_check = 0.

            ( Copy $a and $b respectively )
            var i32 $a_copy = #copy_a.
            var i32 $b_copy = #copy_b.

            var i32 $next_a = $a. (The copy source is consumed here)
            $result = $result + $a_copy. (Add to the result)
            var i32 $a = $next_a. (Restore the original variable)

            var i32 $next_b = $b - 1. (Subtraction. The original $b is consumed here)
            $b = $next_b. (Restore the original variable)
        }.
    } catch 1 {
        ( Come here when $b becomes 0. At this time, $a is alive but $b is consumed )
        var i32 $unused = $a. (Discard the remaining $a)
    }.
    $multiply = $result.
}.

func i32 #main = {
    var i32 $x = 3.
    var i32 $y = 5.
    var i32 $ans = #multiply($x, $y). ( 3 * 5 = 15 )
}.
```

### 8.2. Keyboard Input Echo
Input characters from standard input and continue outputting them as they are until a newline code (`A`) is input.

```fuga
(Helper function to copy variables)
func u8 #copy_char = { $copy_char = $char. }.

func i32 #echo_line = {
    try {
        loop {
            ( Input 1 character. Terminate on exception FFFF0001 (EOF/Error) )
            var u8 $char = #_getc.

            ( Copy the character code and use it for the newline check )
            var u8 $char_copy = #copy_char.

            ( If it is the newline code A, throw exception 2 and end the loop )
            throw 2 @ $char_copy = A.

            ( Output 1 character )
            var i32 $unused = #_putc($char).
        }.
    } catch FFFF0001 {
        ( Processing on EOF or input error )
    } catch 2 {
        ( Termination by detecting a newline. Output a newline at the end )
        var i32 $unused = #_putc(A).
    }.
    $echo_line = 0.
}.
```
