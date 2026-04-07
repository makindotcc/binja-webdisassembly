const fs = require("fs");
const path = require("path");

const decomp = require("./example_wasm.js");
const exoticDecomp = require("./exotic_decompiled.js");

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
let exoticWasmExports;
// Captured log_str calls: { wasm: [...], js: [...] }
const logCapture = { wasm: [], js: [] };

const exoticWasmPath = path.join(__dirname, "exotic.wasm");
if (!fs.existsSync(exoticWasmPath)) {
  console.error(
    "Exotic WASM file not found. Build with: wasm-tools parse tests/exotic.wat -o tests/exotic.wasm"
  );
  process.exit(1);
}

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

async function loadExoticWasm() {
  const wasmBytes = fs.readFileSync(exoticWasmPath);
  const wasmModule = await WebAssembly.compile(wasmBytes);
  const instance = await WebAssembly.instantiate(wasmModule, {});
  exoticWasmExports = instance.exports;
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

// ============================================================
// Exotic instruction tests (from hand-written WAT)
// ============================================================

// Helper: compare floats, handling NaN
function assert_float_eq(actual, expected, label) {
  if (Number.isNaN(expected)) {
    if (Number.isNaN(actual)) {
      passed++;
    } else {
      failed++;
      failures.push({ label, actual, expected: "NaN" });
      console.log(`  FAIL: ${label} — got ${actual}, expected NaN`);
    }
  } else {
    assert_eq(actual, expected, label);
  }
}

// Shorthand for exotic triple-assert (wasm vs expected, js vs expected, js vs wasm)
function exoticTriple(fn, args, expected, label) {
  const wasm = exoticWasmExports[fn](...args);
  const js = exoticDecomp[fn](...args);
  if (typeof expected === "number" && Number.isNaN(expected)) {
    assert_float_eq(wasm, expected, `wasm ${label}`);
    assert_float_eq(js, expected, `js   ${label}`);
    // both NaN => match
    if (Number.isNaN(wasm) && Number.isNaN(js)) { passed++; }
    else { assert_eq(js, wasm, `match ${label}`); }
  } else {
    assert_eq(wasm, expected, `wasm ${label}`);
    assert_eq(js, expected, `js   ${label}`);
    assert_eq(js, wasm, `match ${label}`);
  }
}

function testExoticSelect() {
  section("exotic: select");
  exoticTriple("test_select_i32", [10, 20, 1], 10, "select(10,20,1)");
  exoticTriple("test_select_i32", [10, 20, 0], 20, "select(10,20,0)");
  exoticTriple("test_select_i32", [10, 20, -1], 10, "select(10,20,-1)");
  exoticTriple("test_select_i32", [10, 20, 42], 10, "select(10,20,42)");
  exoticTriple("test_select_i32", [-5, 5, 1], -5, "select(-5,5,1)");
  exoticTriple("test_select_i32", [-5, 5, 0], 5, "select(-5,5,0)");
}

function testExoticAbsViaSelect() {
  section("exotic: abs_via_select");
  exoticTriple("test_abs_via_select", [5], 5, "abs_via_select(5)");
  exoticTriple("test_abs_via_select", [-5], 5, "abs_via_select(-5)");
  exoticTriple("test_abs_via_select", [0], 0, "abs_via_select(0)");
  exoticTriple("test_abs_via_select", [-1], 1, "abs_via_select(-1)");
  exoticTriple("test_abs_via_select", [2147483647], 2147483647, "abs_via_select(MAX)");
}

function testExoticMemory() {
  section("exotic: memory.size / memory.grow");
  // Initial memory size = 1 page
  exoticTriple("test_memory_size", [], 1, "memory_size initial");
  // Grow by 1 page, returns old size (1)
  exoticTriple("test_memory_grow", [1], 1, "memory_grow(1)");
  // Now size should be 2
  exoticTriple("test_memory_size", [], 2, "memory_size after grow");
}

function testExoticBulkMemory() {
  section("exotic: memory.fill / memory.copy");
  // Use high addresses (above 4096) to avoid interference with other tests
  // First clear the area, then fill and verify
  // Fill 8 bytes (more than i32 load width) to ensure clean state
  // 0xABABABAB as signed i32 = -1414812757
  exoticTriple("test_memory_fill_load", [8192, 0xAB, 8], -1414812757, "fill 0xAB load");
  exoticTriple("test_memory_fill_load", [8192, 0xFF, 8], -1, "fill 0xFF load");
  exoticTriple("test_memory_fill_load", [8192, 0x00, 8], 0, "fill 0x00 load");

  // memory.copy: first fill src with known pattern, then copy to dst
  exoticTriple("test_memory_fill_load", [8448, 0x42, 8], 0x42424242, "prep copy src");
  exoticTriple("test_memory_fill_load", [8704, 0x00, 8], 0, "clear copy dst");
  exoticTriple("test_memory_copy_load", [8448, 8704, 4], 0x42424242, "copy and load");
}

function testExoticBrTable() {
  section("exotic: br_table");
  for (let i = 0; i <= 7; i++) {
    exoticTriple("test_br_table", [i], 100 + i, `br_table(${i})`);
  }
  exoticTriple("test_br_table", [8], 999, "br_table(8) default");
  exoticTriple("test_br_table", [100], 999, "br_table(100) default");
  exoticTriple("test_br_table", [-1], 999, "br_table(-1) default");
}

function testExoticNestedBr() {
  section("exotic: nested blocks/loops");
  exoticTriple("test_nested_br", [0], 0, "nested_br(0)");
  exoticTriple("test_nested_br", [1], 1, "nested_br(1)");
  exoticTriple("test_nested_br", [5], 15, "nested_br(5)");
  exoticTriple("test_nested_br", [10], 55, "nested_br(10)");
  exoticTriple("test_nested_br", [100], 5050, "nested_br(100)");
}

function testExoticCallIndirect() {
  section("exotic: call_indirect");
  // op 0 = add, 1 = sub, 2 = mul, 3 = and
  exoticTriple("test_call_indirect", [0, 10, 3], 13, "call_indirect add(10,3)");
  exoticTriple("test_call_indirect", [1, 10, 3], 7, "call_indirect sub(10,3)");
  exoticTriple("test_call_indirect", [2, 10, 3], 30, "call_indirect mul(10,3)");
  exoticTriple("test_call_indirect", [3, 0xFF, 0x0F], 0x0F, "call_indirect and(0xFF,0x0F)");
}

function testExoticI32Unary() {
  section("exotic: i32 clz/ctz/popcnt/eqz");
  // clz
  exoticTriple("test_i32_clz", [0], 32, "clz(0)");
  exoticTriple("test_i32_clz", [1], 31, "clz(1)");
  exoticTriple("test_i32_clz", [-1], 0, "clz(-1)");
  exoticTriple("test_i32_clz", [0x80000000], 0, "clz(0x80000000)");
  exoticTriple("test_i32_clz", [0x00010000], 15, "clz(0x00010000)");
  // ctz
  exoticTriple("test_i32_ctz", [0], 32, "ctz(0)");
  exoticTriple("test_i32_ctz", [1], 0, "ctz(1)");
  exoticTriple("test_i32_ctz", [-1], 0, "ctz(-1)");
  exoticTriple("test_i32_ctz", [0x80000000], 31, "ctz(0x80000000)");
  exoticTriple("test_i32_ctz", [0x100], 8, "ctz(0x100)");
  // popcnt
  exoticTriple("test_i32_popcnt", [0], 0, "popcnt(0)");
  exoticTriple("test_i32_popcnt", [1], 1, "popcnt(1)");
  exoticTriple("test_i32_popcnt", [-1], 32, "popcnt(-1)");
  exoticTriple("test_i32_popcnt", [0x55555555], 16, "popcnt(0x55555555)");
  exoticTriple("test_i32_popcnt", [0xFF], 8, "popcnt(0xFF)");
  // eqz
  exoticTriple("test_i32_eqz", [0], 1, "eqz(0)");
  exoticTriple("test_i32_eqz", [1], 0, "eqz(1)");
  exoticTriple("test_i32_eqz", [-1], 0, "eqz(-1)");
}

function testExoticI32UnsignedCmp() {
  section("exotic: i32 unsigned comparisons");
  // -1 as u32 = 0xFFFFFFFF, which is > any positive
  exoticTriple("test_i32_lt_u", [-1, 1], 0, "lt_u(-1,1)");
  exoticTriple("test_i32_lt_u", [1, -1], 1, "lt_u(1,-1)");
  exoticTriple("test_i32_lt_u", [5, 10], 1, "lt_u(5,10)");
  exoticTriple("test_i32_lt_u", [10, 5], 0, "lt_u(10,5)");
  exoticTriple("test_i32_lt_u", [5, 5], 0, "lt_u(5,5)");

  exoticTriple("test_i32_gt_u", [-1, 1], 1, "gt_u(-1,1)");
  exoticTriple("test_i32_gt_u", [1, -1], 0, "gt_u(1,-1)");
  exoticTriple("test_i32_gt_u", [10, 5], 1, "gt_u(10,5)");

  exoticTriple("test_i32_le_u", [5, 5], 1, "le_u(5,5)");
  exoticTriple("test_i32_le_u", [5, 10], 1, "le_u(5,10)");
  exoticTriple("test_i32_le_u", [-1, 1], 0, "le_u(-1,1)");

  exoticTriple("test_i32_ge_u", [5, 5], 1, "ge_u(5,5)");
  exoticTriple("test_i32_ge_u", [-1, 1], 1, "ge_u(-1,1)");
  exoticTriple("test_i32_ge_u", [1, -1], 0, "ge_u(1,-1)");
}

function testExoticI32Rotate() {
  section("exotic: i32 rotl/rotr");
  exoticTriple("test_i32_rotl", [1, 1], 2, "rotl(1,1)");
  exoticTriple("test_i32_rotl", [1, 31], -2147483648, "rotl(1,31)");
  exoticTriple("test_i32_rotl", [0xFF000000, 8], 0x000000FF, "rotl(0xFF000000,8)");
  exoticTriple("test_i32_rotr", [2, 1], 1, "rotr(2,1)");
  exoticTriple("test_i32_rotr", [1, 1], -2147483648, "rotr(1,1)");
  exoticTriple("test_i32_rotr", [0x000000FF, 8], -16777216, "rotr(0xFF,8)");
}

function testExoticI64Arithmetic() {
  section("exotic: i64 arithmetic");
  exoticTriple("test_i64_add", [1n, 2n], 3n, "i64_add(1,2)");
  exoticTriple("test_i64_add", [-1n, 1n], 0n, "i64_add(-1,1)");
  exoticTriple("test_i64_add", [0x7FFFFFFFFFFFFFFFn, 1n], -9223372036854775808n, "i64_add overflow");

  exoticTriple("test_i64_sub", [10n, 3n], 7n, "i64_sub(10,3)");
  exoticTriple("test_i64_sub", [0n, 1n], -1n, "i64_sub(0,1)");

  exoticTriple("test_i64_mul", [6n, 7n], 42n, "i64_mul(6,7)");
  exoticTriple("test_i64_mul", [-1n, -1n], 1n, "i64_mul(-1,-1)");
  exoticTriple("test_i64_mul", [0x100000000n, 0x100000000n], 0n, "i64_mul overflow");

  exoticTriple("test_i64_div_s", [42n, 6n], 7n, "i64_div_s(42,6)");
  exoticTriple("test_i64_div_s", [-42n, 6n], -7n, "i64_div_s(-42,6)");

  exoticTriple("test_i64_rem_s", [17n, 5n], 2n, "i64_rem_s(17,5)");
  exoticTriple("test_i64_rem_s", [-17n, 5n], -2n, "i64_rem_s(-17,5)");

  exoticTriple("test_i64_div_u", [42n, 6n], 7n, "i64_div_u(42,6)");
  exoticTriple("test_i64_rem_u", [17n, 5n], 2n, "i64_rem_u(17,5)");
}

function testExoticI64Bitwise() {
  section("exotic: i64 bitwise");
  exoticTriple("test_i64_and", [0xFFn, 0x0Fn], 0x0Fn, "i64_and(0xFF,0x0F)");
  exoticTriple("test_i64_and", [-1n, 0n], 0n, "i64_and(-1,0)");

  exoticTriple("test_i64_or", [0xF0n, 0x0Fn], 0xFFn, "i64_or(0xF0,0x0F)");
  exoticTriple("test_i64_or", [0n, 0n], 0n, "i64_or(0,0)");

  exoticTriple("test_i64_xor", [0xFFn, 0xFFn], 0n, "i64_xor(0xFF,0xFF)");
  exoticTriple("test_i64_xor", [0xFFn, 0x00n], 0xFFn, "i64_xor(0xFF,0x00)");

  exoticTriple("test_i64_shl", [1n, 32n], 0x100000000n, "i64_shl(1,32)");
  exoticTriple("test_i64_shl", [1n, 63n], -9223372036854775808n, "i64_shl(1,63)");

  exoticTriple("test_i64_shr_s", [-1n, 32n], -1n, "i64_shr_s(-1,32)");
  exoticTriple("test_i64_shr_s", [-9223372036854775808n, 63n], -1n, "i64_shr_s(MIN,63)");

  exoticTriple("test_i64_shr_u", [-1n, 32n], 0xFFFFFFFFn, "i64_shr_u(-1,32)");
  exoticTriple("test_i64_shr_u", [-9223372036854775808n, 63n], 1n, "i64_shr_u(MIN,63)");

  exoticTriple("test_i64_rotl", [1n, 1n], 2n, "i64_rotl(1,1)");
  exoticTriple("test_i64_rotl", [1n, 63n], -9223372036854775808n, "i64_rotl(1,63)");

  exoticTriple("test_i64_rotr", [2n, 1n], 1n, "i64_rotr(2,1)");
  exoticTriple("test_i64_rotr", [1n, 1n], -9223372036854775808n, "i64_rotr(1,1)");
}

function testExoticI64Compare() {
  section("exotic: i64 comparisons");
  exoticTriple("test_i64_eq", [0n, 0n], 1, "i64_eq(0,0)");
  exoticTriple("test_i64_eq", [1n, 2n], 0, "i64_eq(1,2)");
  exoticTriple("test_i64_ne", [1n, 2n], 1, "i64_ne(1,2)");
  exoticTriple("test_i64_ne", [5n, 5n], 0, "i64_ne(5,5)");

  exoticTriple("test_i64_lt_s", [-1n, 0n], 1, "i64_lt_s(-1,0)");
  exoticTriple("test_i64_lt_s", [0n, -1n], 0, "i64_lt_s(0,-1)");

  exoticTriple("test_i64_lt_u", [-1n, 0n], 0, "i64_lt_u(-1,0)");
  exoticTriple("test_i64_lt_u", [0n, -1n], 1, "i64_lt_u(0,-1)");

  exoticTriple("test_i64_gt_s", [1n, 0n], 1, "i64_gt_s(1,0)");
  exoticTriple("test_i64_gt_s", [-1n, 0n], 0, "i64_gt_s(-1,0)");

  exoticTriple("test_i64_gt_u", [-1n, 0n], 1, "i64_gt_u(-1,0)");

  exoticTriple("test_i64_le_s", [5n, 5n], 1, "i64_le_s(5,5)");
  exoticTriple("test_i64_le_s", [5n, 6n], 1, "i64_le_s(5,6)");
  exoticTriple("test_i64_le_s", [6n, 5n], 0, "i64_le_s(6,5)");

  exoticTriple("test_i64_ge_u", [5n, 5n], 1, "i64_ge_u(5,5)");
  exoticTriple("test_i64_ge_u", [-1n, 0n], 1, "i64_ge_u(-1,0)");
  exoticTriple("test_i64_ge_u", [0n, -1n], 0, "i64_ge_u(0,-1)");
}

function testExoticI64Unary() {
  section("exotic: i64 clz/ctz/popcnt/eqz");
  exoticTriple("test_i64_clz", [0n], 64n, "i64_clz(0)");
  exoticTriple("test_i64_clz", [1n], 63n, "i64_clz(1)");
  exoticTriple("test_i64_clz", [-1n], 0n, "i64_clz(-1)");
  exoticTriple("test_i64_clz", [0x100000000n], 31n, "i64_clz(0x100000000)");

  exoticTriple("test_i64_ctz", [0n], 64n, "i64_ctz(0)");
  exoticTriple("test_i64_ctz", [1n], 0n, "i64_ctz(1)");
  exoticTriple("test_i64_ctz", [-1n], 0n, "i64_ctz(-1)");
  exoticTriple("test_i64_ctz", [0x100000000n], 32n, "i64_ctz(0x100000000)");

  exoticTriple("test_i64_popcnt", [0n], 0n, "i64_popcnt(0)");
  exoticTriple("test_i64_popcnt", [-1n], 64n, "i64_popcnt(-1)");
  exoticTriple("test_i64_popcnt", [0x5555555555555555n], 32n, "i64_popcnt(0x5555...)");

  exoticTriple("test_i64_eqz", [0n], 1, "i64_eqz(0)");
  exoticTriple("test_i64_eqz", [1n], 0, "i64_eqz(1)");
  exoticTriple("test_i64_eqz", [-1n], 0, "i64_eqz(-1)");
}

function testExoticF64Unary() {
  section("exotic: f64 unary");
  exoticTriple("test_f64_abs", [3.14], 3.14, "f64_abs(3.14)");
  exoticTriple("test_f64_abs", [-3.14], 3.14, "f64_abs(-3.14)");
  exoticTriple("test_f64_abs", [0.0], 0.0, "f64_abs(0)");
  exoticTriple("test_f64_abs", [Infinity], Infinity, "f64_abs(Inf)");
  exoticTriple("test_f64_abs", [-Infinity], Infinity, "f64_abs(-Inf)");

  exoticTriple("test_f64_neg", [1.0], -1.0, "f64_neg(1)");
  exoticTriple("test_f64_neg", [-1.0], 1.0, "f64_neg(-1)");
  exoticTriple("test_f64_neg", [0.0], -0.0, "f64_neg(0)");

  exoticTriple("test_f64_ceil", [3.2], 4.0, "f64_ceil(3.2)");
  exoticTriple("test_f64_ceil", [-3.2], -3.0, "f64_ceil(-3.2)");
  exoticTriple("test_f64_ceil", [3.0], 3.0, "f64_ceil(3.0)");

  exoticTriple("test_f64_floor", [3.7], 3.0, "f64_floor(3.7)");
  exoticTriple("test_f64_floor", [-3.2], -4.0, "f64_floor(-3.2)");
  exoticTriple("test_f64_floor", [3.0], 3.0, "f64_floor(3.0)");

  exoticTriple("test_f64_trunc", [3.7], 3.0, "f64_trunc(3.7)");
  exoticTriple("test_f64_trunc", [-3.7], -3.0, "f64_trunc(-3.7)");

  // nearest uses banker's rounding, avoid .5 values
  exoticTriple("test_f64_nearest", [3.3], 3.0, "f64_nearest(3.3)");
  exoticTriple("test_f64_nearest", [3.7], 4.0, "f64_nearest(3.7)");
  exoticTriple("test_f64_nearest", [-3.3], -3.0, "f64_nearest(-3.3)");
  exoticTriple("test_f64_nearest", [-3.7], -4.0, "f64_nearest(-3.7)");

  exoticTriple("test_f64_sqrt", [4.0], 2.0, "f64_sqrt(4)");
  exoticTriple("test_f64_sqrt", [9.0], 3.0, "f64_sqrt(9)");
  exoticTriple("test_f64_sqrt", [0.0], 0.0, "f64_sqrt(0)");
  exoticTriple("test_f64_sqrt", [1.0], 1.0, "f64_sqrt(1)");
}

function testExoticF64Binary() {
  section("exotic: f64 min/max/copysign");
  exoticTriple("test_f64_min", [3.0, 5.0], 3.0, "f64_min(3,5)");
  exoticTriple("test_f64_min", [5.0, 3.0], 3.0, "f64_min(5,3)");
  exoticTriple("test_f64_min", [-1.0, 1.0], -1.0, "f64_min(-1,1)");

  exoticTriple("test_f64_max", [3.0, 5.0], 5.0, "f64_max(3,5)");
  exoticTriple("test_f64_max", [5.0, 3.0], 5.0, "f64_max(5,3)");
  exoticTriple("test_f64_max", [-1.0, 1.0], 1.0, "f64_max(-1,1)");

  exoticTriple("test_f64_copysign", [5.0, -1.0], -5.0, "f64_copysign(5,-1)");
  exoticTriple("test_f64_copysign", [-5.0, 1.0], 5.0, "f64_copysign(-5,1)");
  exoticTriple("test_f64_copysign", [5.0, 1.0], 5.0, "f64_copysign(5,1)");
}

function testExoticF64Compare() {
  section("exotic: f64 comparisons");
  exoticTriple("test_f64_eq", [1.0, 1.0], 1, "f64_eq(1,1)");
  exoticTriple("test_f64_eq", [1.0, 2.0], 0, "f64_eq(1,2)");
  exoticTriple("test_f64_eq", [NaN, NaN], 0, "f64_eq(NaN,NaN)");

  exoticTriple("test_f64_ne", [1.0, 2.0], 1, "f64_ne(1,2)");
  exoticTriple("test_f64_ne", [1.0, 1.0], 0, "f64_ne(1,1)");
  exoticTriple("test_f64_ne", [NaN, NaN], 1, "f64_ne(NaN,NaN)");

  exoticTriple("test_f64_lt", [1.0, 2.0], 1, "f64_lt(1,2)");
  exoticTriple("test_f64_lt", [2.0, 1.0], 0, "f64_lt(2,1)");
  exoticTriple("test_f64_lt", [1.0, 1.0], 0, "f64_lt(1,1)");

  exoticTriple("test_f64_gt", [2.0, 1.0], 1, "f64_gt(2,1)");
  exoticTriple("test_f64_gt", [1.0, 2.0], 0, "f64_gt(1,2)");

  exoticTriple("test_f64_le", [1.0, 1.0], 1, "f64_le(1,1)");
  exoticTriple("test_f64_le", [1.0, 2.0], 1, "f64_le(1,2)");
  exoticTriple("test_f64_le", [2.0, 1.0], 0, "f64_le(2,1)");

  exoticTriple("test_f64_ge", [1.0, 1.0], 1, "f64_ge(1,1)");
  exoticTriple("test_f64_ge", [2.0, 1.0], 1, "f64_ge(2,1)");
  exoticTriple("test_f64_ge", [1.0, 2.0], 0, "f64_ge(1,2)");
}

function testExoticF32Arithmetic() {
  section("exotic: f32 arithmetic");
  const f = Math.fround;
  // f32 functions return f32 (JS number), but might lose precision
  exoticTriple("test_f32_add", [f(1.5), f(2.5)], f(4.0), "f32_add(1.5,2.5)");
  exoticTriple("test_f32_sub", [f(5.0), f(3.0)], f(2.0), "f32_sub(5,3)");
  exoticTriple("test_f32_mul", [f(3.0), f(4.0)], f(12.0), "f32_mul(3,4)");
  exoticTriple("test_f32_div", [f(10.0), f(4.0)], f(2.5), "f32_div(10,4)");
}

function testExoticTypeConversions() {
  section("exotic: type conversions");
  // i32.wrap_i64 — low 32 bits
  exoticTriple("test_i32_wrap_i64", [0x100000001n], 1, "wrap_i64(0x100000001)");
  exoticTriple("test_i32_wrap_i64", [0n], 0, "wrap_i64(0)");
  exoticTriple("test_i32_wrap_i64", [-1n], -1, "wrap_i64(-1)");

  // i64.extend_i32_s — sign-extends
  exoticTriple("test_i64_extend_i32_s", [-1], -1n, "extend_i32_s(-1)");
  exoticTriple("test_i64_extend_i32_s", [42], 42n, "extend_i32_s(42)");
  exoticTriple("test_i64_extend_i32_s", [-2147483648], -2147483648n, "extend_i32_s(MIN)");

  // i64.extend_i32_u — zero-extends
  exoticTriple("test_i64_extend_i32_u", [-1], 0xFFFFFFFFn, "extend_i32_u(-1)");
  exoticTriple("test_i64_extend_i32_u", [42], 42n, "extend_i32_u(42)");

  // Saturating truncations
  exoticTriple("test_i32_trunc_sat_f64_s", [3.7], 3, "trunc_sat_f64_s(3.7)");
  exoticTriple("test_i32_trunc_sat_f64_s", [-3.7], -3, "trunc_sat_f64_s(-3.7)");
  exoticTriple("test_i32_trunc_sat_f64_s", [1e20], 2147483647, "trunc_sat_f64_s(1e20) clamp");
  exoticTriple("test_i32_trunc_sat_f64_s", [-1e20], -2147483648, "trunc_sat_f64_s(-1e20) clamp");
  exoticTriple("test_i32_trunc_sat_f64_s", [NaN], 0, "trunc_sat_f64_s(NaN)");

  exoticTriple("test_i32_trunc_sat_f64_u", [3.7], 3, "trunc_sat_f64_u(3.7)");
  exoticTriple("test_i32_trunc_sat_f64_u", [-1.0], 0, "trunc_sat_f64_u(-1) clamp");
  exoticTriple("test_i32_trunc_sat_f64_u", [NaN], 0, "trunc_sat_f64_u(NaN)");

  // i64 saturating truncation
  exoticTriple("test_i64_trunc_sat_f64_s", [42.9], 42n, "i64_trunc_sat_f64_s(42.9)");
  exoticTriple("test_i64_trunc_sat_f64_s", [-42.9], -42n, "i64_trunc_sat_f64_s(-42.9)");
  exoticTriple("test_i64_trunc_sat_f64_s", [NaN], 0n, "i64_trunc_sat_f64_s(NaN)");

  // f64.convert_i32_s
  exoticTriple("test_f64_convert_i32_s", [42], 42.0, "f64_convert_i32_s(42)");
  exoticTriple("test_f64_convert_i32_s", [-1], -1.0, "f64_convert_i32_s(-1)");

  // f64.convert_i32_u — treats input as unsigned
  exoticTriple("test_f64_convert_i32_u", [42], 42.0, "f64_convert_i32_u(42)");
  exoticTriple("test_f64_convert_i32_u", [-1], 4294967295.0, "f64_convert_i32_u(-1)");

  // f64.convert_i64_s
  exoticTriple("test_f64_convert_i64_s", [42n], 42.0, "f64_convert_i64_s(42)");
  exoticTriple("test_f64_convert_i64_s", [-1n], -1.0, "f64_convert_i64_s(-1)");

  // f32.convert_i32_s
  exoticTriple("test_f32_convert_i32_s", [42], Math.fround(42), "f32_convert_i32_s(42)");
}

function testExoticSignExtensions() {
  section("exotic: sign extensions");
  // i32.extend8_s — sign-extend lowest byte
  exoticTriple("test_i32_extend8_s", [0x7F], 127, "extend8_s(0x7F)");
  exoticTriple("test_i32_extend8_s", [0x80], -128, "extend8_s(0x80)");
  exoticTriple("test_i32_extend8_s", [0xFF], -1, "extend8_s(0xFF)");
  exoticTriple("test_i32_extend8_s", [0x100], 0, "extend8_s(0x100)");

  // i32.extend16_s — sign-extend lowest 16 bits
  exoticTriple("test_i32_extend16_s", [0x7FFF], 32767, "extend16_s(0x7FFF)");
  exoticTriple("test_i32_extend16_s", [0x8000], -32768, "extend16_s(0x8000)");
  exoticTriple("test_i32_extend16_s", [0xFFFF], -1, "extend16_s(0xFFFF)");
  exoticTriple("test_i32_extend16_s", [0x10000], 0, "extend16_s(0x10000)");

  // i64.extend8_s
  exoticTriple("test_i64_extend8_s", [0x7Fn], 127n, "i64_extend8_s(0x7F)");
  exoticTriple("test_i64_extend8_s", [0x80n], -128n, "i64_extend8_s(0x80)");
  exoticTriple("test_i64_extend8_s", [0xFFn], -1n, "i64_extend8_s(0xFF)");

  // i64.extend16_s
  exoticTriple("test_i64_extend16_s", [0x7FFFn], 32767n, "i64_extend16_s(0x7FFF)");
  exoticTriple("test_i64_extend16_s", [0x8000n], -32768n, "i64_extend16_s(0x8000)");

  // i64.extend32_s
  exoticTriple("test_i64_extend32_s", [0x7FFFFFFFn], 2147483647n, "i64_extend32_s(0x7FFFFFFF)");
  exoticTriple("test_i64_extend32_s", [0x80000000n], -2147483648n, "i64_extend32_s(0x80000000)");
  exoticTriple("test_i64_extend32_s", [0xFFFFFFFFn], -1n, "i64_extend32_s(0xFFFFFFFF)");
}

function testExoticSubWordMem() {
  section("exotic: sub-word loads/stores");
  // Using addr=1024 (safe area in memory)
  // store8 + load8_s: 0x80 = 128 unsigned, -128 signed
  exoticTriple("test_store8_load8_s", [1024, 0x7F], 127, "store8_load8_s(0x7F)");
  exoticTriple("test_store8_load8_s", [1024, 0x80], -128, "store8_load8_s(0x80)");
  exoticTriple("test_store8_load8_s", [1024, 0xFF], -1, "store8_load8_s(0xFF)");

  // store8 + load8_u
  exoticTriple("test_store8_load8_u", [1024, 0x7F], 127, "store8_load8_u(0x7F)");
  exoticTriple("test_store8_load8_u", [1024, 0x80], 128, "store8_load8_u(0x80)");
  exoticTriple("test_store8_load8_u", [1024, 0xFF], 255, "store8_load8_u(0xFF)");

  // store16 + load16_s
  exoticTriple("test_store16_load16_s", [1024, 0x7FFF], 32767, "store16_load16_s(0x7FFF)");
  exoticTriple("test_store16_load16_s", [1024, 0x8000], -32768, "store16_load16_s(0x8000)");
  exoticTriple("test_store16_load16_s", [1024, 0xFFFF], -1, "store16_load16_s(0xFFFF)");

  // store16 + load16_u
  exoticTriple("test_store16_load16_u", [1024, 0x7FFF], 32767, "store16_load16_u(0x7FFF)");
  exoticTriple("test_store16_load16_u", [1024, 0x8000], 32768, "store16_load16_u(0x8000)");
  exoticTriple("test_store16_load16_u", [1024, 0xFFFF], 65535, "store16_load16_u(0xFFFF)");
}

function testExoticReinterpret() {
  section("exotic: reinterpret");
  // f32 <-> i32 reinterpret
  // 0x3F800000 = 1.0f
  exoticTriple("test_f32_reinterpret_i32", [0x3F800000], Math.fround(1.0), "f32_reinterpret(0x3F800000)");
  exoticTriple("test_f32_reinterpret_i32", [0], Math.fround(0.0), "f32_reinterpret(0)");
  exoticTriple("test_i32_reinterpret_f32", [Math.fround(1.0)], 0x3F800000, "i32_reinterpret(1.0f)");
  exoticTriple("test_i32_reinterpret_f32", [Math.fround(0.0)], 0, "i32_reinterpret(0.0f)");

  // f64 <-> i64 reinterpret
  // 0x3FF0000000000000 = 1.0
  exoticTriple("test_f64_reinterpret_i64", [0x3FF0000000000000n], 1.0, "f64_reinterpret(1.0 bits)");
  exoticTriple("test_f64_reinterpret_i64", [0n], 0.0, "f64_reinterpret(0)");
  exoticTriple("test_i64_reinterpret_f64", [1.0], 0x3FF0000000000000n, "i64_reinterpret(1.0)");
  exoticTriple("test_i64_reinterpret_f64", [0.0], 0n, "i64_reinterpret(0.0)");
}

function testExoticPromoteDemote() {
  section("exotic: f64.promote / f32.demote");
  exoticTriple("test_f64_promote_f32", [Math.fround(3.14)], Math.fround(3.14), "promote(3.14f)");
  exoticTriple("test_f64_promote_f32", [Math.fround(0.0)], 0.0, "promote(0.0f)");
  exoticTriple("test_f64_promote_f32", [Math.fround(Infinity)], Infinity, "promote(Inf)");

  exoticTriple("test_f32_demote_f64", [3.14], Math.fround(3.14), "demote(3.14)");
  exoticTriple("test_f32_demote_f64", [0.0], Math.fround(0.0), "demote(0.0)");
  exoticTriple("test_f32_demote_f64", [Infinity], Math.fround(Infinity), "demote(Inf)");
}

function testExoticMisc() {
  section("exotic: nop / drop");
  exoticTriple("test_nop", [42], 42, "nop(42)");
  exoticTriple("test_nop", [0], 0, "nop(0)");
  exoticTriple("test_drop_and_return", [10, 99], 10, "drop_and_return(10,99)");
  exoticTriple("test_drop_and_return", [0, -1], 0, "drop_and_return(0,-1)");
}

function testExoticFibLoop() {
  section("exotic: fibonacci (iterative loop)");
  const fibs = [0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55];
  for (let i = 0; i < fibs.length; i++) {
    exoticTriple("test_fib_loop", [i], fibs[i], `fib_loop(${i})`);
  }
  exoticTriple("test_fib_loop", [20], 6765, "fib_loop(20)");
}

function testExoticFib64Loop() {
  section("exotic: fibonacci i64 (iterative loop)");
  const fibs = [0n, 1n, 1n, 2n, 3n, 5n, 8n, 13n, 21n, 34n, 55n];
  for (let i = 0; i < fibs.length; i++) {
    exoticTriple("test_fib64_loop", [i], fibs[i], `fib64_loop(${i})`);
  }
  exoticTriple("test_fib64_loop", [50], 12586269025n, "fib64_loop(50)");
  exoticTriple("test_fib64_loop", [80], 23416728348467685n, "fib64_loop(80)");
}

async function main() {
  await loadWasm();
  await loadExoticWasm();
  console.log("Running tests: WASM vs decompiled JS\n");

  // Original example-wasm tests
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

  // Exotic instruction tests (from WAT)
  testExoticSelect();
  testExoticAbsViaSelect();
  testExoticMemory();
  testExoticBulkMemory();
  testExoticBrTable();
  testExoticNestedBr();
  testExoticCallIndirect();
  testExoticI32Unary();
  testExoticI32UnsignedCmp();
  testExoticI32Rotate();
  testExoticI64Arithmetic();
  testExoticI64Bitwise();
  testExoticI64Compare();
  testExoticI64Unary();
  testExoticF64Unary();
  testExoticF64Binary();
  testExoticF64Compare();
  testExoticF32Arithmetic();
  testExoticTypeConversions();
  testExoticSignExtensions();
  testExoticSubWordMem();
  testExoticReinterpret();
  testExoticPromoteDemote();
  testExoticMisc();
  testExoticFibLoop();
  testExoticFib64Loop();

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
