---
transform: falsy
---

### Skip

Tests that end in `(skip)` in their own header or a parent header are marked as skip

````markdown
## A

### A1

```
a1?
```

```
a1!
```

### A2 (skip)

```
a2?
```

```
a2!
```

### A3

```
a3?
```

```
a3!
```

## B (skip)

### B1

```
b1?
```

```
b1!
```

### B2

```
b2?
```

```
b2!
```

### B3

```
b3?
```

```
b3!
```

## C

### C1

```
c1?
```

```
c1!
```

### C2

```
c2?
```

```
c2!
```

### C3 (skip)

```
c3?
```

```
c3!
```
````

```
[
    Test {
        section: "A",
        name: "A1",
        case: "a1?",
        expected: "a1!",
    },
    Test {
        section: "A",
        name: "A2",
        case: "a2?",
        expected: "a2!",
        skip: true,
    },
    Test {
        section: "A",
        name: "A3",
        case: "a3?",
        expected: "a3!",
    },
    Test {
        section: "B",
        name: "B1",
        case: "b1?",
        expected: "b1!",
        skip: true,
    },
    Test {
        section: "B",
        name: "B2",
        case: "b2?",
        expected: "b2!",
        skip: true,
    },
    Test {
        section: "B",
        name: "B3",
        case: "b3?",
        expected: "b3!",
        skip: true,
    },
    Test {
        section: "C",
        name: "C1",
        case: "c1?",
        expected: "c1!",
    },
    Test {
        section: "C",
        name: "C2",
        case: "c2?",
        expected: "c2!",
    },
    Test {
        section: "C",
        name: "C3",
        case: "c3?",
        expected: "c3!",
        skip: true,
    },
]
```

### Lists as expected

````markdown
---
lists-as-expected: true
---

### A

```
case
```

- expected
````

```
[
    Test {
        name: "A",
        case: "case",
        expected: "- expected",
    },
]
```

### Transforms

````markdown
---
transform: basic
---

### A

```
case
```

```
expected
```
````

```
[
    Test {
        name: "A",
        case: "case",
        expected: "expected",
        transform: Basic,
    },
]
```

### Merge standard error

````markdown
---
merge-stderr: true
---

### A

```
case
```

```
expected
```
````

```
[
    Test {
        name: "A",
        case: "case",
        expected: "expected",
        merge_stderr: true,
    },
]
```

### Wildcard lines

````markdown
---
wildcard-lines: true
---

### A

```
case
```

```
expected
```
````

```
[
    Test {
        name: "A",
        case: "case",
        expected: "expected",
        wildcard_lines: true,
    },
]
```
