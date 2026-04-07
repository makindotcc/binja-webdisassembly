(module
  ;; Memory: 1 page (64KB), exported
  (memory (export "memory") 1)

  ;; Function table for call_indirect tests
  (type $binop_t (func (param i32 i32) (result i32)))
  (table 4 funcref)
  (elem (i32.const 0) $tbl_add $tbl_sub $tbl_mul $tbl_and)

  ;; Table helper functions (not exported, used by call_indirect)
  (func $tbl_add (param i32 i32) (result i32)
    (i32.add (local.get 0) (local.get 1)))
  (func $tbl_sub (param i32 i32) (result i32)
    (i32.sub (local.get 0) (local.get 1)))
  (func $tbl_mul (param i32 i32) (result i32)
    (i32.mul (local.get 0) (local.get 1)))
  (func $tbl_and (param i32 i32) (result i32)
    (i32.and (local.get 0) (local.get 1)))

  ;; =====================================================================
  ;; SELECT
  ;; =====================================================================

  (func (export "test_select_i32") (param i32 i32 i32) (result i32)
    (select (local.get 0) (local.get 1) (local.get 2)))

  ;; =====================================================================
  ;; MEMORY MANAGEMENT
  ;; =====================================================================

  (func (export "test_memory_size") (result i32)
    (memory.size))

  (func (export "test_memory_grow") (param i32) (result i32)
    (memory.grow (local.get 0)))

  ;; =====================================================================
  ;; BULK MEMORY
  ;; =====================================================================

  ;; Fills memory[addr..addr+len] with val, then loads i32 at addr
  (func (export "test_memory_fill_load") (param $addr i32) (param $val i32) (param $len i32) (result i32)
    (memory.fill (local.get $addr) (local.get $val) (local.get $len))
    (i32.load (local.get $addr)))

  ;; Copies len bytes from src to dst, then loads i32 at dst
  (func (export "test_memory_copy_load") (param $src i32) (param $dst i32) (param $len i32) (result i32)
    (memory.copy (local.get $dst) (local.get $src) (local.get $len))
    (i32.load (local.get $dst)))

  ;; =====================================================================
  ;; CONTROL FLOW: br_table
  ;; =====================================================================

  ;; Returns 100+idx for idx 0..7, or 999 for out-of-range
  (func (export "test_br_table") (param $idx i32) (result i32)
    (block $b7 (result i32)
      (block $b6 (result i32)
        (block $b5 (result i32)
          (block $b4 (result i32)
            (block $b3 (result i32)
              (block $b2 (result i32)
                (block $b1 (result i32)
                  (block $b0 (result i32)
                    (block $default (result i32)
                      (i32.const 999)
                      (br_table $b0 $b1 $b2 $b3 $b4 $b5 $b6 $b7 $default
                        (local.get $idx)))
                    ;; default
                    (return))
                  ;; b0: idx=0
                  (drop)
                  (i32.const 100)
                  (return))
                ;; b1: idx=1
                (drop)
                (i32.const 101)
                (return))
              ;; b2: idx=2
              (drop)
              (i32.const 102)
              (return))
            ;; b3: idx=3
            (drop)
            (i32.const 103)
            (return))
          ;; b4: idx=4
          (drop)
          (i32.const 104)
          (return))
        ;; b5: idx=5
        (drop)
        (i32.const 105)
        (return))
      ;; b6: idx=6
      (drop)
      (i32.const 106)
      (return))
    ;; b7: idx=7
    (drop)
    (i32.const 107))

  ;; =====================================================================
  ;; CONTROL FLOW: nested blocks/loops
  ;; =====================================================================

  ;; Sums 1..n using nested block+loop+br_if pattern
  (func (export "test_nested_br") (param $n i32) (result i32)
    (local $sum i32)
    (local $i i32)
    (local.set $i (i32.const 1))
    (local.set $sum (i32.const 0))
    (block $outer
      (loop $inner
        ;; if i > n, break
        (br_if $outer (i32.gt_s (local.get $i) (local.get $n)))
        ;; sum += i
        (local.set $sum (i32.add (local.get $sum) (local.get $i)))
        ;; i++
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        ;; continue loop
        (br $inner)))
    (local.get $sum))

  ;; =====================================================================
  ;; CALL INDIRECT
  ;; =====================================================================

  (func (export "test_call_indirect") (param $op i32) (param $a i32) (param $b i32) (result i32)
    (call_indirect (type $binop_t) (local.get $a) (local.get $b) (local.get $op)))

  ;; =====================================================================
  ;; i32 UNARY
  ;; =====================================================================

  (func (export "test_i32_clz") (param i32) (result i32)
    (i32.clz (local.get 0)))

  (func (export "test_i32_ctz") (param i32) (result i32)
    (i32.ctz (local.get 0)))

  (func (export "test_i32_popcnt") (param i32) (result i32)
    (i32.popcnt (local.get 0)))

  (func (export "test_i32_eqz") (param i32) (result i32)
    (i32.eqz (local.get 0)))

  ;; =====================================================================
  ;; i32 UNSIGNED COMPARISONS
  ;; =====================================================================

  (func (export "test_i32_lt_u") (param i32 i32) (result i32)
    (i32.lt_u (local.get 0) (local.get 1)))

  (func (export "test_i32_gt_u") (param i32 i32) (result i32)
    (i32.gt_u (local.get 0) (local.get 1)))

  (func (export "test_i32_le_u") (param i32 i32) (result i32)
    (i32.le_u (local.get 0) (local.get 1)))

  (func (export "test_i32_ge_u") (param i32 i32) (result i32)
    (i32.ge_u (local.get 0) (local.get 1)))

  ;; =====================================================================
  ;; i32 ROTL / ROTR
  ;; =====================================================================

  (func (export "test_i32_rotl") (param i32 i32) (result i32)
    (i32.rotl (local.get 0) (local.get 1)))

  (func (export "test_i32_rotr") (param i32 i32) (result i32)
    (i32.rotr (local.get 0) (local.get 1)))

  ;; =====================================================================
  ;; i64 ARITHMETIC
  ;; =====================================================================

  (func (export "test_i64_add") (param i64 i64) (result i64)
    (i64.add (local.get 0) (local.get 1)))

  (func (export "test_i64_sub") (param i64 i64) (result i64)
    (i64.sub (local.get 0) (local.get 1)))

  (func (export "test_i64_mul") (param i64 i64) (result i64)
    (i64.mul (local.get 0) (local.get 1)))

  (func (export "test_i64_div_s") (param i64 i64) (result i64)
    (i64.div_s (local.get 0) (local.get 1)))

  (func (export "test_i64_rem_s") (param i64 i64) (result i64)
    (i64.rem_s (local.get 0) (local.get 1)))

  (func (export "test_i64_div_u") (param i64 i64) (result i64)
    (i64.div_u (local.get 0) (local.get 1)))

  (func (export "test_i64_rem_u") (param i64 i64) (result i64)
    (i64.rem_u (local.get 0) (local.get 1)))

  ;; =====================================================================
  ;; i64 BITWISE
  ;; =====================================================================

  (func (export "test_i64_and") (param i64 i64) (result i64)
    (i64.and (local.get 0) (local.get 1)))

  (func (export "test_i64_or") (param i64 i64) (result i64)
    (i64.or (local.get 0) (local.get 1)))

  (func (export "test_i64_xor") (param i64 i64) (result i64)
    (i64.xor (local.get 0) (local.get 1)))

  (func (export "test_i64_shl") (param i64 i64) (result i64)
    (i64.shl (local.get 0) (local.get 1)))

  (func (export "test_i64_shr_s") (param i64 i64) (result i64)
    (i64.shr_s (local.get 0) (local.get 1)))

  (func (export "test_i64_shr_u") (param i64 i64) (result i64)
    (i64.shr_u (local.get 0) (local.get 1)))

  (func (export "test_i64_rotl") (param i64 i64) (result i64)
    (i64.rotl (local.get 0) (local.get 1)))

  (func (export "test_i64_rotr") (param i64 i64) (result i64)
    (i64.rotr (local.get 0) (local.get 1)))

  ;; =====================================================================
  ;; i64 COMPARISONS
  ;; =====================================================================

  (func (export "test_i64_eq") (param i64 i64) (result i32)
    (i64.eq (local.get 0) (local.get 1)))

  (func (export "test_i64_ne") (param i64 i64) (result i32)
    (i64.ne (local.get 0) (local.get 1)))

  (func (export "test_i64_lt_s") (param i64 i64) (result i32)
    (i64.lt_s (local.get 0) (local.get 1)))

  (func (export "test_i64_lt_u") (param i64 i64) (result i32)
    (i64.lt_u (local.get 0) (local.get 1)))

  (func (export "test_i64_gt_s") (param i64 i64) (result i32)
    (i64.gt_s (local.get 0) (local.get 1)))

  (func (export "test_i64_gt_u") (param i64 i64) (result i32)
    (i64.gt_u (local.get 0) (local.get 1)))

  (func (export "test_i64_le_s") (param i64 i64) (result i32)
    (i64.le_s (local.get 0) (local.get 1)))

  (func (export "test_i64_ge_u") (param i64 i64) (result i32)
    (i64.ge_u (local.get 0) (local.get 1)))

  ;; =====================================================================
  ;; i64 UNARY
  ;; =====================================================================

  (func (export "test_i64_clz") (param i64) (result i64)
    (i64.clz (local.get 0)))

  (func (export "test_i64_ctz") (param i64) (result i64)
    (i64.ctz (local.get 0)))

  (func (export "test_i64_popcnt") (param i64) (result i64)
    (i64.popcnt (local.get 0)))

  (func (export "test_i64_eqz") (param i64) (result i32)
    (i64.eqz (local.get 0)))

  ;; =====================================================================
  ;; f64 UNARY
  ;; =====================================================================

  (func (export "test_f64_abs") (param f64) (result f64)
    (f64.abs (local.get 0)))

  (func (export "test_f64_neg") (param f64) (result f64)
    (f64.neg (local.get 0)))

  (func (export "test_f64_ceil") (param f64) (result f64)
    (f64.ceil (local.get 0)))

  (func (export "test_f64_floor") (param f64) (result f64)
    (f64.floor (local.get 0)))

  (func (export "test_f64_trunc") (param f64) (result f64)
    (f64.trunc (local.get 0)))

  (func (export "test_f64_nearest") (param f64) (result f64)
    (f64.nearest (local.get 0)))

  (func (export "test_f64_sqrt") (param f64) (result f64)
    (f64.sqrt (local.get 0)))

  ;; =====================================================================
  ;; f64 BINARY
  ;; =====================================================================

  (func (export "test_f64_min") (param f64 f64) (result f64)
    (f64.min (local.get 0) (local.get 1)))

  (func (export "test_f64_max") (param f64 f64) (result f64)
    (f64.max (local.get 0) (local.get 1)))

  (func (export "test_f64_copysign") (param f64 f64) (result f64)
    (f64.copysign (local.get 0) (local.get 1)))

  ;; =====================================================================
  ;; f64 COMPARISONS
  ;; =====================================================================

  (func (export "test_f64_eq") (param f64 f64) (result i32)
    (f64.eq (local.get 0) (local.get 1)))

  (func (export "test_f64_ne") (param f64 f64) (result i32)
    (f64.ne (local.get 0) (local.get 1)))

  (func (export "test_f64_lt") (param f64 f64) (result i32)
    (f64.lt (local.get 0) (local.get 1)))

  (func (export "test_f64_gt") (param f64 f64) (result i32)
    (f64.gt (local.get 0) (local.get 1)))

  (func (export "test_f64_le") (param f64 f64) (result i32)
    (f64.le (local.get 0) (local.get 1)))

  (func (export "test_f64_ge") (param f64 f64) (result i32)
    (f64.ge (local.get 0) (local.get 1)))

  ;; =====================================================================
  ;; f32 ARITHMETIC
  ;; =====================================================================

  (func (export "test_f32_add") (param f32 f32) (result f32)
    (f32.add (local.get 0) (local.get 1)))

  (func (export "test_f32_sub") (param f32 f32) (result f32)
    (f32.sub (local.get 0) (local.get 1)))

  (func (export "test_f32_mul") (param f32 f32) (result f32)
    (f32.mul (local.get 0) (local.get 1)))

  (func (export "test_f32_div") (param f32 f32) (result f32)
    (f32.div (local.get 0) (local.get 1)))

  ;; =====================================================================
  ;; TYPE CONVERSIONS
  ;; =====================================================================

  (func (export "test_i32_wrap_i64") (param i64) (result i32)
    (i32.wrap_i64 (local.get 0)))

  (func (export "test_i64_extend_i32_s") (param i32) (result i64)
    (i64.extend_i32_s (local.get 0)))

  (func (export "test_i64_extend_i32_u") (param i32) (result i64)
    (i64.extend_i32_u (local.get 0)))

  (func (export "test_i32_trunc_sat_f64_s") (param f64) (result i32)
    (i32.trunc_sat_f64_s (local.get 0)))

  (func (export "test_i32_trunc_sat_f64_u") (param f64) (result i32)
    (i32.trunc_sat_f64_u (local.get 0)))

  (func (export "test_i64_trunc_sat_f64_s") (param f64) (result i64)
    (i64.trunc_sat_f64_s (local.get 0)))

  (func (export "test_f64_convert_i32_s") (param i32) (result f64)
    (f64.convert_i32_s (local.get 0)))

  (func (export "test_f64_convert_i32_u") (param i32) (result f64)
    (f64.convert_i32_u (local.get 0)))

  (func (export "test_f64_convert_i64_s") (param i64) (result f64)
    (f64.convert_i64_s (local.get 0)))

  (func (export "test_f32_convert_i32_s") (param i32) (result f32)
    (f32.convert_i32_s (local.get 0)))

  ;; =====================================================================
  ;; SIGN EXTENSIONS
  ;; =====================================================================

  (func (export "test_i32_extend8_s") (param i32) (result i32)
    (i32.extend8_s (local.get 0)))

  (func (export "test_i32_extend16_s") (param i32) (result i32)
    (i32.extend16_s (local.get 0)))

  (func (export "test_i64_extend8_s") (param i64) (result i64)
    (i64.extend8_s (local.get 0)))

  (func (export "test_i64_extend16_s") (param i64) (result i64)
    (i64.extend16_s (local.get 0)))

  (func (export "test_i64_extend32_s") (param i64) (result i64)
    (i64.extend32_s (local.get 0)))

  ;; =====================================================================
  ;; SUB-WORD LOADS/STORES
  ;; =====================================================================

  ;; Store byte at addr, load back as signed i8
  (func (export "test_store8_load8_s") (param $addr i32) (param $val i32) (result i32)
    (i32.store8 (local.get $addr) (local.get $val))
    (i32.load8_s (local.get $addr)))

  ;; Store byte at addr, load back as unsigned u8
  (func (export "test_store8_load8_u") (param $addr i32) (param $val i32) (result i32)
    (i32.store8 (local.get $addr) (local.get $val))
    (i32.load8_u (local.get $addr)))

  ;; Store i16 at addr, load back as signed
  (func (export "test_store16_load16_s") (param $addr i32) (param $val i32) (result i32)
    (i32.store16 (local.get $addr) (local.get $val))
    (i32.load16_s (local.get $addr)))

  ;; Store i16 at addr, load back as unsigned
  (func (export "test_store16_load16_u") (param $addr i32) (param $val i32) (result i32)
    (i32.store16 (local.get $addr) (local.get $val))
    (i32.load16_u (local.get $addr)))

  ;; =====================================================================
  ;; REINTERPRET
  ;; =====================================================================

  (func (export "test_f32_reinterpret_i32") (param i32) (result f32)
    (f32.reinterpret_i32 (local.get 0)))

  (func (export "test_i32_reinterpret_f32") (param f32) (result i32)
    (i32.reinterpret_f32 (local.get 0)))

  (func (export "test_f64_reinterpret_i64") (param i64) (result f64)
    (f64.reinterpret_i64 (local.get 0)))

  (func (export "test_i64_reinterpret_f64") (param f64) (result i64)
    (i64.reinterpret_f64 (local.get 0)))

  ;; =====================================================================
  ;; PROMOTE / DEMOTE
  ;; =====================================================================

  (func (export "test_f64_promote_f32") (param f32) (result f64)
    (f64.promote_f32 (local.get 0)))

  (func (export "test_f32_demote_f64") (param f64) (result f32)
    (f32.demote_f64 (local.get 0)))

  ;; =====================================================================
  ;; MISC: nop, drop
  ;; =====================================================================

  (func (export "test_nop") (param i32) (result i32)
    (nop)
    (local.get 0))

  ;; Pushes y onto stack, drops it, returns x
  (func (export "test_drop_and_return") (param $x i32) (param $y i32) (result i32)
    (drop (local.get $y))
    (local.get $x))

  ;; =====================================================================
  ;; COMBINED: exercises multiple instructions in one function
  ;; =====================================================================

  ;; Absolute value of i32 using select: select(x, -x, x >= 0)
  (func (export "test_abs_via_select") (param $x i32) (result i32)
    (select
      (local.get $x)
      (i32.sub (i32.const 0) (local.get $x))
      (i32.ge_s (local.get $x) (i32.const 0))))

  ;; Fibonacci using loop (iterative, exercises locals + loop + br_if)
  (func (export "test_fib_loop") (param $n i32) (result i32)
    (local $a i32)
    (local $b i32)
    (local $i i32)
    (local $tmp i32)
    (local.set $a (i32.const 0))
    (local.set $b (i32.const 1))
    (local.set $i (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_s (local.get $i) (local.get $n)))
        (local.set $tmp (local.get $b))
        (local.set $b (i32.add (local.get $a) (local.get $b)))
        (local.set $a (local.get $tmp))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (local.get $a))

  ;; i64 fibonacci (iterative) - exercises i64 add + loop
  (func (export "test_fib64_loop") (param $n i32) (result i64)
    (local $a i64)
    (local $b i64)
    (local $i i32)
    (local $tmp i64)
    (local.set $a (i64.const 0))
    (local.set $b (i64.const 1))
    (local.set $i (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_s (local.get $i) (local.get $n)))
        (local.set $tmp (local.get $b))
        (local.set $b (i64.add (local.get $a) (local.get $b)))
        (local.set $a (local.get $tmp))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (local.get $a))
)
