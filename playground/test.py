import ranim, timeit

def py_matrix_add(a, b):
    return [[a[i][j] + b[i][j] for j in range(len(a[0]))] for i in range(len(a))]

def py_matrix_add_for(a, b):
    c = a.copy()
    for i in range(len(a)):
        for j in range(len(a[0])):
            c[i][j] = a[i][j] + b[i][j]
    return c

def rust_matrix_add(a, b):
    return ranim.sum_matrix(a, b)

for k in range(3):
    print("-"*10)
    print(f"benchmarking {10**k}x{10**k} matrix...")
    a = [[i for i in range(10**k)] for j in range(10**k)]
    b = [[i for i in range(10**k)] for j in range(10**k)]

    py_time = timeit.timeit(lambda: py_matrix_add(a, b), number=10000)
    print(f"py_matrix_add time: {py_time}")

    py_time = timeit.timeit(lambda: py_matrix_add_for(a, b), number=10000)
    print(f"py_matrix_add_for time: {py_time}")

    rust_time = timeit.timeit(lambda: rust_matrix_add(a, b), number=10000)
    print(f"rust_matrix_add time: {rust_time}")
