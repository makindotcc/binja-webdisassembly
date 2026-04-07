#[unsafe(no_mangle)]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[unsafe(no_mangle)]
pub extern "C" fn factorial(n: i32) -> i32 {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}

#[unsafe(no_mangle)]
pub extern "C" fn fibonacci(n: i32) -> i32 {
    if n <= 1 {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn is_prime(n: i32) -> i32 {
    if n <= 1 {
        return 0;
    }
    if n <= 3 {
        return 1;
    }
    if n % 2 == 0 || n % 3 == 0 {
        return 0;
    }
    let mut i = 5;
    while i * i <= n {
        if n % i == 0 || n % (i + 2) == 0 {
            return 0;
        }
        i += 6;
    }
    1
}

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    /// Callback to JS host: log a string (ptr + len in linear memory)
    fn log_str(ptr: *const u8, len: usize);
}

#[unsafe(no_mangle)]
pub extern "C" fn string_starts_with_hello(foo: *const u8, len: usize) {
    let slice = unsafe { core::slice::from_raw_parts(foo, len) };
    if let Ok(string) = core::str::from_utf8(slice) {
        if string.starts_with("Hello") {
            let msg = "String starts with 'Hello'";
            unsafe { log_str(msg.as_ptr(), msg.len()) };
        } else {
            let msg = "String does not start with 'Hello'";
            unsafe { log_str(msg.as_ptr(), msg.len()) };
        }
    }
}

/// Returns 1 if string starts with "Hello", 0 otherwise
#[unsafe(no_mangle)]
pub extern "C" fn check_starts_with_hello(ptr: *const u8, len: usize) -> i32 {
    let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
    if let Ok(string) = core::str::from_utf8(slice) {
        if string.starts_with("Hello") {
            return 1;
        }
    }
    0
}

static mut COUNTER: i32 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn increment() -> i32 {
    unsafe {
        COUNTER += 1;
        COUNTER
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_counter() -> i32 {
    unsafe { COUNTER }
}

#[unsafe(no_mangle)]
pub extern "C" fn reset_counter() {
    unsafe {
        COUNTER = 0;
    }
}

// ──────────────────────────────────────────────────────────────────────
// Advanced: f64 math
// ──────────────────────────────────────────────────────────────────────

/// Newton's method sqrt, exercises f64 arithmetic + loops
#[unsafe(no_mangle)]
pub extern "C" fn sqrt_newton(x: f64) -> f64 {
    if x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return 0.0;
    }
    let mut guess = x / 2.0;
    let mut i = 0;
    while i < 50 {
        let next = (guess + x / guess) * 0.5;
        if (next - guess).abs() < 1e-15 {
            break;
        }
        guess = next;
        i += 1;
    }
    guess
}

/// Distance between two 2D points — f64 multiply, add, sqrt
#[unsafe(no_mangle)]
pub extern "C" fn distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    sqrt_newton(dx * dx + dy * dy)
}

// ──────────────────────────────────────────────────────────────────────
// Advanced: i64 + bitwise
// ──────────────────────────────────────────────────────────────────────

/// Population count (Hamming weight) for i64
#[unsafe(no_mangle)]
pub extern "C" fn popcount64(mut x: i64) -> i32 {
    let mut count = 0i32;
    while x != 0 {
        count += 1;
        x &= x - 1;
    }
    count
}

/// Rotate left for i64
#[unsafe(no_mangle)]
pub extern "C" fn rotl64(x: i64, k: i32) -> i64 {
    let k = (k & 63) as u32;
    ((x as u64).rotate_left(k)) as i64
}

/// Simple 64-bit hash (xorshift64)
#[unsafe(no_mangle)]
pub extern "C" fn hash64(mut x: i64) -> i64 {
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

// ──────────────────────────────────────────────────────────────────────
// Advanced: sorting in linear memory
// ──────────────────────────────────────────────────────────────────────

/// Bubble sort an i32 array in linear memory. ptr = address, len = element count.
#[unsafe(no_mangle)]
pub extern "C" fn bubble_sort(ptr: *mut i32, len: i32) {
    if len <= 1 {
        return;
    }
    let slice = unsafe { core::slice::from_raw_parts_mut(ptr, len as usize) };
    let n = slice.len();
    for i in 0..n {
        let mut swapped = false;
        for j in 0..n - 1 - i {
            if slice[j] > slice[j + 1] {
                slice.swap(j, j + 1);
                swapped = true;
            }
        }
        if !swapped {
            break;
        }
    }
}

/// Sum i32 array in linear memory
#[unsafe(no_mangle)]
pub extern "C" fn sum_array(ptr: *const i32, len: i32) -> i32 {
    let slice = unsafe { core::slice::from_raw_parts(ptr, len as usize) };
    let mut sum = 0i32;
    for &x in slice {
        sum = sum.wrapping_add(x);
    }
    sum
}

// ──────────────────────────────────────────────────────────────────────
// Advanced: enum dispatch (match statement → br_table)
// ──────────────────────────────────────────────────────────────────────

/// op: 0=add, 1=sub, 2=mul, 3=div, 4=mod, 5=and, 6=or, 7=xor
#[unsafe(no_mangle)]
pub extern "C" fn calc(op: i32, a: i32, b: i32) -> i32 {
    match op {
        0 => a.wrapping_add(b),
        1 => a.wrapping_sub(b),
        2 => a.wrapping_mul(b),
        3 => {
            if b == 0 {
                0
            } else {
                a.wrapping_div(b)
            }
        }
        4 => {
            if b == 0 {
                0
            } else {
                a.wrapping_rem(b)
            }
        }
        5 => a & b,
        6 => a | b,
        7 => a ^ b,
        _ => 0,
    }
}

// ──────────────────────────────────────────────────────────────────────
// Advanced: mutual recursion (is_even / is_odd)
// ──────────────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn is_even(n: i32) -> i32 {
    if n == 0 {
        1
    } else if n < 0 {
        is_odd(n + 1)
    } else {
        is_odd(n - 1)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn is_odd(n: i32) -> i32 {
    if n == 0 {
        0
    } else if n < 0 {
        is_even(n + 1)
    } else {
        is_even(n - 1)
    }
}

// ──────────────────────────────────────────────────────────────────────
// Advanced: write results to memory (multi-value via pointer)
// ──────────────────────────────────────────────────────────────────────

/// Divmod: writes quotient and remainder to out_ptr[0] and out_ptr[1]
#[unsafe(no_mangle)]
pub extern "C" fn divmod(a: i32, b: i32, out_ptr: *mut i32) {
    if b == 0 {
        unsafe {
            *out_ptr = 0;
            *out_ptr.add(1) = 0;
        }
    } else {
        unsafe {
            *out_ptr = a / b;
            *out_ptr.add(1) = a % b;
        }
    }
}

/// Matrix 2x2 multiply: reads A (4 i32s at a_ptr), B (4 at b_ptr), writes C (4 at out_ptr)
/// Layout: [row0col0, row0col1, row1col0, row1col1]
#[unsafe(no_mangle)]
pub extern "C" fn mat2_mul(a_ptr: *const i32, b_ptr: *const i32, out_ptr: *mut i32) {
    unsafe {
        let a = core::slice::from_raw_parts(a_ptr, 4);
        let b = core::slice::from_raw_parts(b_ptr, 4);
        let out = core::slice::from_raw_parts_mut(out_ptr, 4);
        out[0] = a[0].wrapping_mul(b[0]).wrapping_add(a[1].wrapping_mul(b[2]));
        out[1] = a[0].wrapping_mul(b[1]).wrapping_add(a[1].wrapping_mul(b[3]));
        out[2] = a[2].wrapping_mul(b[0]).wrapping_add(a[3].wrapping_mul(b[2]));
        out[3] = a[2].wrapping_mul(b[1]).wrapping_add(a[3].wrapping_mul(b[3]));
    }
}

// ──────────────────────────────────────────────────────────────────────
// Advanced: iterative algorithms with complex control flow
// ──────────────────────────────────────────────────────────────────────

/// GCD using Euclidean algorithm
#[unsafe(no_mangle)]
pub extern "C" fn gcd(mut a: i32, mut b: i32) -> i32 {
    if a < 0 { a = -a; }
    if b < 0 { b = -b; }
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Collatz sequence length (how many steps to reach 1)
#[unsafe(no_mangle)]
pub extern "C" fn collatz_steps(mut n: i32) -> i32 {
    if n <= 0 {
        return -1;
    }
    let mut steps = 0;
    while n != 1 {
        if n % 2 == 0 {
            n /= 2;
        } else {
            n = 3 * n + 1;
        }
        steps += 1;
    }
    steps
}

// ──────────────────────────────────────────────────────────────────────
// Advanced: type conversions
// ──────────────────────────────────────────────────────────────────────

/// f64 → i32 truncation (saturating)
#[unsafe(no_mangle)]
pub extern "C" fn f64_to_i32(x: f64) -> i32 {
    x as i32
}

/// i32 → f64
#[unsafe(no_mangle)]
pub extern "C" fn i32_to_f64(x: i32) -> f64 {
    x as f64
}

/// i64 → f64
#[unsafe(no_mangle)]
pub extern "C" fn i64_to_f64(x: i64) -> f64 {
    x as f64
}

/// Reinterpret i32 bits as f32
#[unsafe(no_mangle)]
pub extern "C" fn i32_bits_to_f32(x: i32) -> f32 {
    f32::from_bits(x as u32)
}
