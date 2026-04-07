const fs = require("fs");
const path = require("path");

const decomp = require("./target/wasm32-unknown-unknown/release/example_wasm.wasm");

const wasmPath = path.join(
  __dirname,
  "../target/wasm32-unknown-unknown/release/example_wasm.wasm"
);
if (!fs.existsSync(wasmPath)) {
  console.error(
    "WASM file not found. Build with: cargo build -p example-wasm --target wasm32-unknown-unknown --release"
  );
  process.exit(1);
}

let wasmExports;
// Captured log_str calls: { wasm: [...], js: [...] }
const logCapture = { wasm: [], js: [] };

async function loadWasm() {
  const wasmBytes = fs.readFileSync(wasmPath);
  const wasmModule = await WebAssembly.compile(wasmBytes);
  const instance = await WebAssembly.instantiate(wasmModule, {
    env: {
      log_str(ptr, len) {
        const bytes = new Uint8Array(instance.exports.memory.buffer, ptr, len);
        logCapture.wasm.push(new TextDecoder().decode(bytes));
      },
    },
  });
  wasmExports = instance.exports;

  // Hook decompiled JS imports
  decomp.imports.env_log_str = (ptr, len) => {
    const bytes = new Uint8Array(decomp.memory.buffer, ptr, len);
    logCapture.js.push(new TextDecoder().decode(bytes));
  };
}

let passed = 0;
let failed = 0;
const failures = [];

function assert_eq(actual, expected, label) {
  if (actual === expected) {
    passed++;
  } else {
    failed++;
    failures.push({ label, actual, expected });
    console.log(`  FAIL: ${label} — got ${actual}, expected ${expected}`);
  }
}

function section(name) {
  console.log(`\n── ${name} ──`);
}

function testAdd() {
  section("add");
  const cases = [
    [0, 0, 0],
    [1, 2, 3],
    [-1, 1, 0],
    [100, 200, 300],
    [-50, -30, -80],
    [2147483647, 0, 2147483647], // i32 max
  ];
  for (const [a, b, expected] of cases) {
    const wasm = wasmExports.add(a, b);
    const js = decomp.add(a, b);
    assert_eq(wasm, expected, `wasm add(${a}, ${b})`);
    assert_eq(js, expected, `js   add(${a}, ${b})`);
    assert_eq(js, wasm, `match add(${a}, ${b})`);
  }
}

function testFactorial() {
  section("factorial");
  const cases = [
    [0, 1],
    [1, 1],
    [2, 2],
    [3, 6],
    [4, 24],
    [5, 120],
    [6, 720],
    [7, 5040],
    [10, 3628800],
    [12, 479001600],
  ];
  for (const [n, expected] of cases) {
    const wasm = wasmExports.factorial(n);
    const js = decomp.factorial(n);
    assert_eq(wasm, expected, `wasm factorial(${n})`);
    assert_eq(js, expected, `js   factorial(${n})`);
    assert_eq(js, wasm, `match factorial(${n})`);
  }
}

function testFibonacci() {
  section("fibonacci");
  const cases = [
    [0, 0],
    [1, 1],
    [2, 1],
    [3, 2],
    [4, 3],
    [5, 5],
    [6, 8],
    [7, 13],
    [8, 21],
    [10, 55],
    [15, 610],
    [20, 6765],
  ];
  for (const [n, expected] of cases) {
    const wasm = wasmExports.fibonacci(n);
    const js = decomp.fibonacci(n);
    assert_eq(wasm, expected, `wasm fibonacci(${n})`);
    assert_eq(js, expected, `js   fibonacci(${n})`);
    assert_eq(js, wasm, `match fibonacci(${n})`);
  }
}

function testIsPrime() {
  section("is_prime");
  const cases = [
    [0, 0],
    [1, 0],
    [2, 1],
    [3, 1],
    [4, 0],
    [5, 1],
    [6, 0],
    [7, 1],
    [10, 0],
    [11, 1],
    [13, 1],
    [15, 0],
    [17, 1],
    [23, 1],
    [25, 0],
    [29, 1],
    [49, 0],
    [97, 1],
    [100, 0],
    [101, 1],
    [997, 1],
    [1000, 0],
  ];
  for (const [n, expected] of cases) {
    const wasm = wasmExports.is_prime(n);
    const js = decomp.is_prime(n);
    assert_eq(wasm, expected, `wasm is_prime(${n})`);
    assert_eq(js, expected, `js   is_prime(${n})`);
    assert_eq(js, wasm, `match is_prime(${n})`);
  }
}

function testCounter() {
  section("counter (increment / get_counter / reset_counter)");

  // The decompiled counter uses memory at address 1052504.
  // The WASM counter uses its own linear memory.
  // We test each independently.

  // WASM side
  wasmExports.reset_counter();
  assert_eq(wasmExports.get_counter(), 0, "wasm get_counter() after reset");
  assert_eq(wasmExports.increment(), 1, "wasm increment() -> 1");
  assert_eq(wasmExports.increment(), 2, "wasm increment() -> 2");
  assert_eq(wasmExports.increment(), 3, "wasm increment() -> 3");
  assert_eq(wasmExports.get_counter(), 3, "wasm get_counter() == 3");
  wasmExports.reset_counter();
  assert_eq(wasmExports.get_counter(), 0, "wasm get_counter() after 2nd reset");

  // JS decompiled side
  decomp.reset_counter();
  assert_eq(decomp.get_counter(), 0, "js get_counter() after reset");
  assert_eq(decomp.increment(), 1, "js increment() -> 1");
  assert_eq(decomp.increment(), 2, "js increment() -> 2");
  assert_eq(decomp.increment(), 3, "js increment() -> 3");
  assert_eq(decomp.get_counter(), 3, "js get_counter() == 3");
  decomp.reset_counter();
  assert_eq(decomp.get_counter(), 0, "js get_counter() after 2nd reset");
}

function testCheckStartsWithHello() {
  section("check_starts_with_hello");
  const encoder = new TextEncoder();

  // Helper: write string into WASM memory and call check_starts_with_hello
  function wasmCheck(str) {
    const bytes = encoder.encode(str);
    // Write into WASM linear memory above heap base
    const heapBase = wasmExports.__heap_base.value;
    const addr = heapBase + 4096;
    new Uint8Array(wasmExports.memory.buffer).set(bytes, addr);
    return wasmExports.check_starts_with_hello(addr, bytes.length);
  }

  // Helper: write string into decompiled JS memory and call check_starts_with_hello
  function jsCheck(str) {
    const bytes = encoder.encode(str);
    const addr = 2048; // safe address in linear memory (below data segments)
    new Uint8Array(decomp.memory.buffer).set(bytes, addr);
    return decomp.check_starts_with_hello(addr, bytes.length);
  }

  const cases = [
    ["Hello, world!", 1],
    ["Hello", 1],
    ["Hello!", 1],
    ["Hellothere", 1],
    ["Hell", 0],
    ["Goodbye", 0],
    ["", 0],
    ["H", 0],
    ["He", 0],
    ["Hel", 0],
    ["hell", 0],      // case sensitive
    ["hELLO", 0],     // case sensitive
    ["HELLO", 0],     // case sensitive
    ["Hello World", 1],
    ["AHello", 0],    // doesn't start with Hello
    [" Hello", 0],    // leading space
  ];

  for (const [str, expected] of cases) {
    const wasm = wasmCheck(str);
    const js = jsCheck(str);
    assert_eq(wasm, expected, `wasm check_starts_with_hello("${str}")`);
    assert_eq(js, expected, `js   check_starts_with_hello("${str}")`);
    assert_eq(js, wasm, `match check_starts_with_hello("${str}")`);
  }
}

function testStringStartsWithHello() {
  section("string_starts_with_hello (callback + stack pointer integrity)");
  const encoder = new TextEncoder();

  // Helper for WASM side
  function wasmCall(str) {
    const bytes = encoder.encode(str);
    const heapBase = wasmExports.__heap_base.value;
    const addr = heapBase + 4096;
    new Uint8Array(wasmExports.memory.buffer).set(bytes, addr);
    logCapture.wasm = [];
    try {
      wasmExports.string_starts_with_hello(addr, bytes.length);
      return { ok: true, log: logCapture.wasm.slice() };
    } catch (e) {
      return { ok: false, error: e.message, log: [] };
    }
  }

  // Helper for decompiled JS side
  function jsCall(str) {
    const bytes = encoder.encode(str);
    const addr = 2048;
    new Uint8Array(decomp.memory.buffer).set(bytes, addr);
    logCapture.js = [];
    const g0Before = decomp.globals.g0;
    try {
      decomp.string_starts_with_hello(addr, bytes.length);
    } catch (e) {
      return { ok: false, error: e.message, log: [], g0Restored: false };
    }
    const g0After = decomp.globals.g0;
    return { ok: true, log: logCapture.js.slice(), g0Restored: g0Before === g0After };
  }

  const cases = [
    ["Hello, world!", "String starts with 'Hello'"],
    ["Hello", "String starts with 'Hello'"],
    ["Goodbye", "String does not start with 'Hello'"],
    ["Hell", "String does not start with 'Hello'"],
    ["Hello!!!", "String starts with 'Hello'"],
    ["A longer string", "String does not start with 'Hello'"],
  ];

  for (const [str, expectedMsg] of cases) {
    const wasmResult = wasmCall(str);
    const jsResult = jsCall(str);
    assert_eq(wasmResult.ok, true, `wasm string_starts_with_hello("${str}") no crash`);
    assert_eq(jsResult.ok, true, `js   string_starts_with_hello("${str}") no crash`);
    // Verify callback was invoked with the right message
    assert_eq(wasmResult.log.length, 1, `wasm string_starts_with_hello("${str}") log count`);
    assert_eq(jsResult.log.length, 1, `js   string_starts_with_hello("${str}") log count`);
    assert_eq(wasmResult.log[0], expectedMsg, `wasm string_starts_with_hello("${str}") message`);
    assert_eq(jsResult.log[0], expectedMsg, `js   string_starts_with_hello("${str}") message`);
    assert_eq(jsResult.log[0], wasmResult.log[0], `match string_starts_with_hello("${str}") message`);
    assert_eq(jsResult.g0Restored, true, `js   string_starts_with_hello("${str}") g0 restored`);
  }

  // Empty string and invalid utf8-length edge case: no callback expected
  for (const str of ["", "H"]) {
    const wasmResult = wasmCall(str);
    const jsResult = jsCall(str);
    assert_eq(wasmResult.ok, true, `wasm string_starts_with_hello("${str}") no crash`);
    assert_eq(jsResult.ok, true, `js   string_starts_with_hello("${str}") no crash`);
    assert_eq(wasmResult.log.length, jsResult.log.length, `match string_starts_with_hello("${str}") log count`);
  }
}

function testCalc() {
  section("calc");
  // op: 0=add, 1=sub, 2=mul, 3=div, 4=mod, 5=and, 6=or, 7=xor
  const cases = [
    [0, 10, 3, 13],     // add
    [1, 10, 3, 7],      // sub
    [2, 10, 3, 30],     // mul
    [3, 10, 3, 3],      // div
    [3, 10, 0, 0],      // div by zero
    [4, 10, 3, 1],      // mod
    [4, 10, 0, 0],      // mod by zero
    [5, 0xFF, 0x0F, 0x0F], // and
    [6, 0xF0, 0x0F, 0xFF], // or
    [7, 0xFF, 0x0F, 0xF0], // xor
    [8, 1, 2, 0],       // unknown op
    [1, 0, 0, 0],       // sub zeros
    [2, -1, -1, 1],     // mul negatives
  ];
  for (const [op, a, b, expected] of cases) {
    const wasm = wasmExports.calc(op, a, b);
    const js = decomp.calc(op, a, b);
    assert_eq(wasm, expected, `wasm calc(${op}, ${a}, ${b})`);
    assert_eq(js, expected, `js   calc(${op}, ${a}, ${b})`);
    assert_eq(js, wasm, `match calc(${op}, ${a}, ${b})`);
  }
}

function testIsEvenOdd() {
  section("is_even / is_odd");
  const cases = [
    [0, 1, 0],
    [1, 0, 1],
    [2, 1, 0],
    [3, 0, 1],
    [4, 1, 0],
    [5, 0, 1],
    [10, 1, 0],
    [11, 0, 1],
    [-1, 0, 1],
    [-2, 1, 0],
    [-3, 0, 1],
    [-4, 1, 0],
  ];
  for (const [n, expEven, expOdd] of cases) {
    const wasmEven = wasmExports.is_even(n);
    const jsEven = decomp.is_even(n);
    assert_eq(wasmEven, expEven, `wasm is_even(${n})`);
    assert_eq(jsEven, expEven, `js   is_even(${n})`);
    assert_eq(jsEven, wasmEven, `match is_even(${n})`);

    const wasmOdd = wasmExports.is_odd(n);
    const jsOdd = decomp.is_odd(n);
    assert_eq(wasmOdd, expOdd, `wasm is_odd(${n})`);
    assert_eq(jsOdd, expOdd, `js   is_odd(${n})`);
    assert_eq(jsOdd, wasmOdd, `match is_odd(${n})`);
  }
}

function testGcd() {
  section("gcd");
  const cases = [
    [0, 0, 0],
    [1, 0, 1],
    [0, 1, 1],
    [6, 4, 2],
    [12, 8, 4],
    [17, 13, 1],
    [100, 75, 25],
    [48, 18, 6],
    [-12, 8, 4],
    [12, -8, 4],
    [-12, -8, 4],
  ];
  for (const [a, b, expected] of cases) {
    const wasm = wasmExports.gcd(a, b);
    const js = decomp.gcd(a, b);
    assert_eq(wasm, expected, `wasm gcd(${a}, ${b})`);
    assert_eq(js, expected, `js   gcd(${a}, ${b})`);
    assert_eq(js, wasm, `match gcd(${a}, ${b})`);
  }
}

function testCollatzSteps() {
  section("collatz_steps");
  const cases = [
    [1, 0],
    [2, 1],
    [3, 7],
    [4, 2],
    [5, 5],
    [6, 8],
    [7, 16],
    [8, 3],
    [10, 6],
    [27, 111],
    [0, -1],
    [-1, -1],
  ];
  for (const [n, expected] of cases) {
    const wasm = wasmExports.collatz_steps(n);
    const js = decomp.collatz_steps(n);
    assert_eq(wasm, expected, `wasm collatz_steps(${n})`);
    assert_eq(js, expected, `js   collatz_steps(${n})`);
    assert_eq(js, wasm, `match collatz_steps(${n})`);
  }
}

function testSqrtNewton() {
  section("sqrt_newton");
  const cases = [
    [0, 0],
    [1, 1],
    [4, 2],
    [9, 3],
    [16, 4],
    [25, 5],
    [100, 10],
    [2, Math.sqrt(2)],
    [0.25, 0.5],
  ];
  for (const [x, expected] of cases) {
    const wasm = wasmExports.sqrt_newton(x);
    const js = decomp.sqrt_newton(x);
    assert_eq(Math.abs(wasm - expected) < 1e-10, true, `wasm sqrt_newton(${x}) ≈ ${expected}`);
    assert_eq(Math.abs(js - expected) < 1e-10, true, `js   sqrt_newton(${x}) ≈ ${expected}`);
    assert_eq(Math.abs(js - wasm) < 1e-10, true, `match sqrt_newton(${x})`);
  }
  // Negative → NaN
  const wasmNeg = wasmExports.sqrt_newton(-1);
  const jsNeg = decomp.sqrt_newton(-1);
  assert_eq(Number.isNaN(wasmNeg), true, "wasm sqrt_newton(-1) is NaN");
  assert_eq(Number.isNaN(jsNeg), true, "js   sqrt_newton(-1) is NaN");
}

function testDistance() {
  section("distance");
  const cases = [
    [0, 0, 0, 0, 0],
    [0, 0, 3, 4, 5],
    [1, 1, 4, 5, 5],
    [0, 0, 1, 0, 1],
    [-3, -4, 0, 0, 5],
  ];
  for (const [x1, y1, x2, y2, expected] of cases) {
    const wasm = wasmExports.distance(x1, y1, x2, y2);
    const js = decomp.distance(x1, y1, x2, y2);
    assert_eq(Math.abs(wasm - expected) < 1e-10, true, `wasm distance(${x1},${y1},${x2},${y2}) ≈ ${expected}`);
    assert_eq(Math.abs(js - expected) < 1e-10, true, `js   distance(${x1},${y1},${x2},${y2}) ≈ ${expected}`);
    assert_eq(Math.abs(js - wasm) < 1e-10, true, `match distance(${x1},${y1},${x2},${y2})`);
  }
}

function testF64ToI32() {
  section("f64_to_i32");
  const cases = [
    [0.0, 0],
    [3.7, 3],
    [-3.7, -3],
    [100.99, 100],
    [-0.5, 0],
  ];
  for (const [x, expected] of cases) {
    const wasm = wasmExports.f64_to_i32(x);
    const js = decomp.f64_to_i32(x);
    assert_eq(wasm, expected, `wasm f64_to_i32(${x})`);
    assert_eq(js, expected, `js   f64_to_i32(${x})`);
    assert_eq(js, wasm, `match f64_to_i32(${x})`);
  }
}

function testI32ToF64() {
  section("i32_to_f64");
  const cases = [0, 1, -1, 42, 2147483647, -2147483648];
  for (const x of cases) {
    const wasm = wasmExports.i32_to_f64(x);
    const js = decomp.i32_to_f64(x);
    assert_eq(wasm, x, `wasm i32_to_f64(${x})`);
    assert_eq(js, x, `js   i32_to_f64(${x})`);
    assert_eq(js, wasm, `match i32_to_f64(${x})`);
  }
}

function testI64ToF64() {
  section("i64_to_f64");
  const cases = [0n, 1n, -1n, 42n, 1000000n, -1000000n];
  for (const x of cases) {
    const expected = Number(x);
    const wasm = wasmExports.i64_to_f64(x);
    const js = decomp.i64_to_f64(x);
    assert_eq(wasm, expected, `wasm i64_to_f64(${x})`);
    assert_eq(js, expected, `js   i64_to_f64(${x})`);
    assert_eq(js, wasm, `match i64_to_f64(${x})`);
  }
}

function testI32BitsToF32() {
  section("i32_bits_to_f32");
  // 0x3f800000 = 1.0 in IEEE 754 float
  // 0x40000000 = 2.0
  // 0x00000000 = 0.0
  const cases = [
    [0x3f800000, 1.0],
    [0x40000000, 2.0],
    [0x00000000, 0.0],
    [0xbf800000 | 0, -1.0], // | 0 to make it a signed i32
    [0x40490fdb | 0, Math.fround(Math.PI)],
  ];
  for (const [bits, expected] of cases) {
    const wasm = wasmExports.i32_bits_to_f32(bits);
    const js = decomp.i32_bits_to_f32(bits);
    assert_eq(Math.abs(wasm - expected) < 1e-6, true, `wasm i32_bits_to_f32(0x${(bits >>> 0).toString(16)}) ≈ ${expected}`);
    assert_eq(Math.abs(js - expected) < 1e-6, true, `js   i32_bits_to_f32(0x${(bits >>> 0).toString(16)}) ≈ ${expected}`);
    assert_eq(wasm, js, `match i32_bits_to_f32(0x${(bits >>> 0).toString(16)})`);
  }
}

function testPopcount64() {
  section("popcount64");
  const cases = [
    [0n, 0],
    [1n, 1],
    [0xFFn, 8],
    [0xFFFFFFFFn, 32],
    [0x8000000000000000n, 1],
    [0xFFFFFFFFFFFFFFFFn, 64],
    [0xAAAAAAAAAAAAAAAAn, 32],
  ];
  for (const [x, expected] of cases) {
    const wasm = wasmExports.popcount64(x);
    const js = decomp.popcount64(x);
    assert_eq(wasm, expected, `wasm popcount64(0x${x.toString(16)})`);
    assert_eq(js, expected, `js   popcount64(0x${x.toString(16)})`);
    assert_eq(js, wasm, `match popcount64(0x${x.toString(16)})`);
  }
}

function testRotl64() {
  section("rotl64");
  const cases = [
    [1n, 0],
    [1n, 1],
    [1n, 63],
    [0x123456789ABCDEFn, 4],
    [0xFFn, 8],
    [-1n, 17],
  ];
  for (const [x, k] of cases) {
    const wasm = wasmExports.rotl64(x, k);
    const js = decomp.rotl64(x, k);
    assert_eq(js, wasm, `match rotl64(0x${(x >= 0n ? x : BigInt.asUintN(64, x)).toString(16)}, ${k})`);
  }
  // Verify basic properties
  assert_eq(wasmExports.rotl64(1n, 0), 1n, "wasm rotl64(1, 0) == 1");
  assert_eq(wasmExports.rotl64(1n, 1), 2n, "wasm rotl64(1, 1) == 2");
}

function testHash64() {
  section("hash64");
  // hash64 is deterministic; just verify wasm == js for various inputs
  const inputs = [0n, 1n, -1n, 42n, 0x123456789ABCDEF0n, 0x7FFFFFFFFFFFFFFFn];
  for (const x of inputs) {
    const wasm = wasmExports.hash64(x);
    const js = decomp.hash64(x);
    assert_eq(js, wasm, `match hash64(0x${(x < 0n ? (-x).toString(16) : x.toString(16))})`);
  }
  // Verify non-trivial: hash64(x) != x for nonzero inputs
  assert_eq(wasmExports.hash64(42n) !== 42n, true, "wasm hash64(42) != 42");
  assert_eq(decomp.hash64(42n) !== 42n, true, "js   hash64(42) != 42");
}

function testSumArray() {
  section("sum_array");

  function writeI32Array(exports, addr, arr) {
    const buf = new DataView(exports.memory ? exports.memory.buffer : exports.buffer);
    for (let i = 0; i < arr.length; i++) {
      buf.setInt32(addr + i * 4, arr[i], true);
    }
  }

  const heapBase = wasmExports.__heap_base.value;
  const addr = heapBase + 4096;
  const jsAddr = 2048;

  const cases = [
    [[], 0],
    [[1], 1],
    [[1, 2, 3], 6],
    [[10, 20, 30, 40, 50], 150],
    [[-1, -2, -3, -4], -10],
    [[1, -1, 2, -2, 3, -3], 0],
    [[100, 200, 300, 400, 500, 600, 700, 800, 900, 1000], 5500],
  ];
  for (const [arr, expected] of cases) {
    writeI32Array(wasmExports, addr, arr);
    writeI32Array({ buffer: decomp.memory.buffer }, jsAddr, arr);
    const wasm = wasmExports.sum_array(addr, arr.length);
    const js = decomp.sum_array(jsAddr, arr.length);
    assert_eq(wasm, expected, `wasm sum_array([${arr}])`);
    assert_eq(js, expected, `js   sum_array([${arr}])`);
    assert_eq(js, wasm, `match sum_array([${arr}])`);
  }
}

function testBubbleSort() {
  section("bubble_sort");

  function readI32Array(mem, addr, len) {
    const dv = new DataView(mem);
    return Array.from({ length: len }, (_, i) => dv.getInt32(addr + i * 4, true));
  }
  function writeI32Array(mem, addr, arr) {
    const dv = new DataView(mem);
    for (let i = 0; i < arr.length; i++) dv.setInt32(addr + i * 4, arr[i], true);
  }

  const heapBase = wasmExports.__heap_base.value;
  const addr = heapBase + 4096;
  const jsAddr = 2048;

  const cases = [
    [[], []],
    [[1], [1]],
    [[3, 1, 2], [1, 2, 3]],
    [[5, 4, 3, 2, 1], [1, 2, 3, 4, 5]],
    [[1, 2, 3, 4], [1, 2, 3, 4]], // already sorted
    [[-3, 0, -1, 2, -2, 1], [-3, -2, -1, 0, 1, 2]],
    [[7, 7, 7], [7, 7, 7]], // all equal
    [[42, 1], [1, 42]],
  ];
  for (const [input, expected] of cases) {
    writeI32Array(wasmExports.memory.buffer, addr, input);
    wasmExports.bubble_sort(addr, input.length);
    const wasmResult = readI32Array(wasmExports.memory.buffer, addr, input.length);

    writeI32Array(decomp.memory.buffer, jsAddr, input);
    decomp.bubble_sort(jsAddr, input.length);
    const jsResult = readI32Array(decomp.memory.buffer, jsAddr, input.length);

    assert_eq(JSON.stringify(wasmResult), JSON.stringify(expected), `wasm bubble_sort([${input}])`);
    assert_eq(JSON.stringify(jsResult), JSON.stringify(expected), `js   bubble_sort([${input}])`);
    assert_eq(JSON.stringify(jsResult), JSON.stringify(wasmResult), `match bubble_sort([${input}])`);
  }
}

function testDivmod() {
  section("divmod");

  function readI32Pair(mem, addr) {
    const dv = new DataView(mem);
    return [dv.getInt32(addr, true), dv.getInt32(addr + 4, true)];
  }

  const heapBase = wasmExports.__heap_base.value;
  const addr = heapBase + 4096;
  const jsAddr = 2048;

  const cases = [
    [10, 3, [3, 1]],
    [7, 2, [3, 1]],
    [0, 5, [0, 0]],
    [5, 5, [1, 0]],
    [-10, 3, [-3, -1]],
    [10, -3, [-3, 1]],
    [10, 0, [0, 0]],  // div by zero → 0,0
  ];
  for (const [a, b, expected] of cases) {
    wasmExports.divmod(a, b, addr);
    const wasmResult = readI32Pair(wasmExports.memory.buffer, addr);

    decomp.divmod(a, b, jsAddr);
    const jsResult = readI32Pair(decomp.memory.buffer, jsAddr);

    assert_eq(JSON.stringify(wasmResult), JSON.stringify(expected), `wasm divmod(${a}, ${b})`);
    assert_eq(JSON.stringify(jsResult), JSON.stringify(expected), `js   divmod(${a}, ${b})`);
    assert_eq(JSON.stringify(jsResult), JSON.stringify(wasmResult), `match divmod(${a}, ${b})`);
  }
}

function testMat2Mul() {
  section("mat2_mul");

  function writeI32Array(mem, addr, arr) {
    const dv = new DataView(mem);
    for (let i = 0; i < arr.length; i++) dv.setInt32(addr + i * 4, arr[i], true);
  }
  function readI32Array(mem, addr, len) {
    const dv = new DataView(mem);
    return Array.from({ length: len }, (_, i) => dv.getInt32(addr + i * 4, true));
  }

  const heapBase = wasmExports.__heap_base.value;
  const wasmA = heapBase + 4096;
  const wasmB = wasmA + 16;
  const wasmOut = wasmB + 16;
  const jsA = 2048;
  const jsB = jsA + 16;
  const jsOut = jsB + 16;

  // Identity * Identity = Identity
  const identity = [1, 0, 0, 1];
  const cases = [
    [identity, identity, identity],
    [[1, 2, 3, 4], identity, [1, 2, 3, 4]],
    [identity, [5, 6, 7, 8], [5, 6, 7, 8]],
    [[1, 2, 3, 4], [5, 6, 7, 8], [19, 22, 43, 50]],
    [[2, 0, 0, 2], [3, 0, 0, 3], [6, 0, 0, 6]], // scalar
  ];
  for (const [a, b, expected] of cases) {
    writeI32Array(wasmExports.memory.buffer, wasmA, a);
    writeI32Array(wasmExports.memory.buffer, wasmB, b);
    wasmExports.mat2_mul(wasmA, wasmB, wasmOut);
    const wasmResult = readI32Array(wasmExports.memory.buffer, wasmOut, 4);

    writeI32Array(decomp.memory.buffer, jsA, a);
    writeI32Array(decomp.memory.buffer, jsB, b);
    decomp.mat2_mul(jsA, jsB, jsOut);
    const jsResult = readI32Array(decomp.memory.buffer, jsOut, 4);

    assert_eq(JSON.stringify(wasmResult), JSON.stringify(expected), `wasm mat2_mul([${a}],[${b}])`);
    assert_eq(JSON.stringify(jsResult), JSON.stringify(expected), `js   mat2_mul([${a}],[${b}])`);
    assert_eq(JSON.stringify(jsResult), JSON.stringify(wasmResult), `match mat2_mul([${a}],[${b}])`);
  }
}

async function main() {
  await loadWasm();
  console.log("Running tests: WASM vs decompiled JS\n");

  testAdd();
  testFactorial();
  testFibonacci();
  testIsPrime();
  testCounter();
  testCheckStartsWithHello();
  testStringStartsWithHello();
  testCalc();
  testIsEvenOdd();
  testGcd();
  testCollatzSteps();
  testSqrtNewton();
  testDistance();
  testF64ToI32();
  testI32ToF64();
  testI64ToF64();
  testI32BitsToF32();
  testPopcount64();
  testRotl64();
  testHash64();
  testSumArray();
  testBubbleSort();
  testDivmod();
  testMat2Mul();

  // Summary
  console.log(`\n${"=".repeat(50)}`);
  console.log(`Results: ${passed} passed, ${failed} failed`);
  if (failures.length > 0) {
    console.log("\nFailures:");
    for (const f of failures) {
      console.log(`  ${f.label}: got ${f.actual}, expected ${f.expected}`);
    }
  }
  console.log("=".repeat(50));
  process.exit(failed > 0 ? 1 : 0);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
