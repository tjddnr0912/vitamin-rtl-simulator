# 02 · SystemVerilog Arrays

Per IEEE 1800-2017 §7. Verilog supported only the one-dimensional fixed array (a memory); SV
adds four more forms — multi-dimensional packed, dynamic, associative and queue.

---

## The five kinds of array

| Kind | Declaration | Size fixed | Key type | Synthesizable |
|------|-------------|------------|----------|---------------|
| Packed | `logic [3:0][7:0] m` | compile time | integer index | yes |
| Unpacked | `int arr [0:7]` | compile time | integer index | yes (simple forms) |
| Dynamic | `int d[]` | run time, `new[N]` | integer index | no |
| Associative | `int aa[string]` | hashed automatically | any type | no |
| Queue | `int q[$]` | run time, push/pop | integer index | no |

---

## Packed multi-dimensional arrays

The declared ranges sit after the type and before the identifier — the array maps onto one
contiguous bit vector.

```systemverilog
logic [3:0][7:0] matrix;   // 4 × 8 bits = one contiguous 32-bit vector
matrix[2][3]               // bit 3 of byte 2
logic [31:0] raw = matrix; // the whole vector can be read at once
```

Bit width: the product of every packed dimension. Synthesis treats it as a single bundle of
wires.

---

## Unpacked arrays

The declared ranges sit after the identifier — nothing guarantees that the elements are
contiguous.

```systemverilog
int arr [0:7];     // 8 integers (Verilog style)
int arr2 [8];      // the same — SV shorthand
int mat  [4][8];   // 4 × 8, multi-dimensional (added by SV)
```

Identical to a Verilog one-dimensional memory declaration, extended to several dimensions in SV.

---

## Dynamic Array

An unpacked array whose size is decided at run time. Allocated with `new[N]`.

```systemverilog
int dyn[];
dyn = new[8];            // 8 elements, each defaulting to 0
dyn = new[16](dyn);      // reallocate to 16, copying the existing contents
dyn.delete();            // release everything (size → 0)
int s = dyn.size();      // current size
```

- Accessing it without a `new[N]` is a runtime null-access error.
- Omitting the `(dyn)` copy argument on reallocation discards the existing contents.

---

## Associative Array

A key-value hash map. No memory is allocated at declaration. Suits a sparse address space, or
a flexible key type.

```systemverilog
int aa[string];    // string key
int aa2[int];      // integer key
int aa3[*];        // wildcard — any integer expression as the key
```

### Writing and reading

```systemverilog
aa["alpha"] = 1;
int v = aa["alpha"];
```

### Methods

| Method | Effect |
|--------|--------|
| `.exists(key)` | whether the key exists → returns 1/0 |
| `.delete(key)` | deletes one key |
| `.delete()` | deletes everything |
| `.num()` | returns the number of entries |
| `.first(ref key)` | stores the first key in key. Returns 0 if empty |
| `.last(ref key)` | stores the last key in key |
| `.next(ref key)` | overwrites key with the key after it. Returns 0 at the last key or when empty |
| `.prev(ref key)` | overwrites key with the key before it |

```systemverilog
string k;
if (aa.first(k)) begin
    do begin
        $display("%s = %0d", k, aa[k]);
    end while (aa.next(k));
end
```

---

## Queue

A double-ended dynamic FIFO. A size bound may be given at declaration as `[$:N]` (unbounded if
omitted).

```systemverilog
int q[$];         // unbounded queue
int q2[$:7];      // queue of at most 8 elements
```

### Methods

| Method | Effect |
|--------|--------|
| `.push_back(item)` | appends an element at the back |
| `.push_front(item)` | prepends an element at the front |
| `.pop_back()` | returns and removes the back element |
| `.pop_front()` | returns and removes the front element |
| `.insert(idx, item)` | inserts at index idx |
| `.delete(idx)` | deletes the element at index idx |
| `.delete()` | deletes everything |
| `.size()` | returns the current number of elements |

```systemverilog
int q[$];
q.push_back(10);
q.push_front(5);
q.insert(1, 7);          // [5, 7, 10]
int v = q.pop_front();   // v=5, q=[7, 10]
```

A queue can also be accessed with array-slice syntax: `q[0:2]` yields the elements at indices
0 through 2.

---

## The array method library

These apply alike to dynamic arrays, unpacked arrays and queues.

### Ordering and rearranging

They modify the array elements in place.

| Method | Effect | `with` clause |
|--------|--------|---------------|
| `.sort()` | ascending sort | optional (key expression) |
| `.rsort()` | descending sort | optional |
| `.reverse()` | reverses the order (values unchanged) | not allowed |
| `.shuffle()` | random shuffle | not allowed |

```systemverilog
int a[] = '{5, 3, 8, 1};
a.sort();                         // {1, 3, 5, 8}
a.rsort();                        // {8, 5, 3, 1}
a.sort with (item % 3);           // ascending by remainder
```

### Locating elements

A `with` clause is mandatory and the result is always a queue. When nothing matches, the queue
comes back empty.

| Method | Returns |
|--------|---------|
| `.find with (cond)` | a queue of the matching elements |
| `.find_first with (cond)` | a queue holding the first such element |
| `.find_last with (cond)` | a queue holding the last such element |
| `.find_index with (cond)` | a queue of the matching indices |
| `.find_first_index with (cond)` | a queue holding the first such index |
| `.find_last_index with (cond)` | a queue holding the last such index |

```systemverilog
int a[] = '{1, 5, 3, 8, 2};
int found[$] = a.find with (item > 4);   // {5, 8}
int idx[$]   = a.find_index with (item > 4); // {1, 3}
```

### Reduction

Reduces the whole array to a single scalar. A `with` clause can transform each element before
the reduction.

| Method | Effect |
|--------|--------|
| `.sum()` | sum of all elements |
| `.product()` | product of all elements |
| `.and()` | bitwise AND |
| `.or()` | bitwise OR |
| `.xor()` | bitwise XOR |

```systemverilog
int a[] = '{1, 2, 3, 4};
int s = a.sum();                    // 10
int s2 = a.sum with (item * 2);     // 20 (each element doubled, then summed)
```

---

## Sources

- IEEE 1800-2017 §7 (Aggregate data types)
- chipverify.com/systemverilog/systemverilog-associative-array
- vlsiverify.com/system-verilog/associative-array-in-systemverilog/
- verificationguide.com/systemverilog/systemverilog-queue/
- sagar5258.blogspot.com/2017/09/array-manipulation-methods-in.html
- verificationguide.com/systemverilog/systemverilog-array-ordering-methods/
