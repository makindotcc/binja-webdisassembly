#[unsafe(no_mangle)]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[unsafe(no_mangle)]
pub extern "C" fn factorial(n: i32) -> i32 {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
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
