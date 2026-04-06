(module $example_wasm.wasm
  (type (;0;) (func (param i32 i32)))
  (type (;1;) (func (param i32 i32 i32) (result i32)))
  (type (;2;) (func (param i32 i32) (result i32)))
  (type (;3;) (func (param i32) (result i32)))
  (type (;4;) (func (result i32)))
  (type (;5;) (func))
  (type (;6;) (func (param i32 i32 i32)))
  (type (;7;) (func (param i32 i32 i32 i32) (result i32)))
  (type (;8;) (func (param i32)))
  (type (;9;) (func (param i32 i32 i32 i32 i32)))
  (type (;10;) (func (param i32 i32 i32 i32 i32 i32)))
  (func $add (type 2) (param i32 i32) (result i32)
    local.get 1
    local.get 0
    i32.add)
  (func $factorial (type 3) (param i32) (result i32)
    (local i32 i32 i32 i32 i32)
    i32.const 1
    local.set 1
    block  ;; label = @1
      local.get 0
      i32.const 2
      i32.lt_s
      br_if 0 (;@1;)
      local.get 0
      i32.const -1
      i32.add
      local.tee 2
      i32.const 7
      i32.and
      local.set 3
      block  ;; label = @2
        block  ;; label = @3
          local.get 0
          i32.const -2
          i32.add
          i32.const 7
          i32.ge_u
          br_if 0 (;@3;)
          i32.const 1
          local.set 1
          br 1 (;@2;)
        end
        i32.const 0
        local.set 4
        i32.const 0
        local.get 2
        i32.const -8
        i32.and
        i32.sub
        local.set 5
        i32.const 1
        local.set 1
        loop  ;; label = @3
          local.get 1
          local.get 0
          local.get 4
          i32.add
          local.tee 2
          i32.mul
          local.get 2
          i32.const -1
          i32.add
          i32.mul
          local.get 2
          i32.const -2
          i32.add
          i32.mul
          local.get 2
          i32.const -3
          i32.add
          i32.mul
          local.get 2
          i32.const -4
          i32.add
          i32.mul
          local.get 2
          i32.const -5
          i32.add
          i32.mul
          local.get 2
          i32.const -6
          i32.add
          i32.mul
          local.get 2
          i32.const -7
          i32.add
          i32.mul
          local.set 1
          local.get 5
          local.get 4
          i32.const -8
          i32.add
          local.tee 4
          i32.ne
          br_if 0 (;@3;)
        end
        local.get 0
        local.get 4
        i32.add
        local.set 0
      end
      local.get 3
      i32.eqz
      br_if 0 (;@1;)
      loop  ;; label = @2
        local.get 1
        local.get 0
        i32.mul
        local.set 1
        local.get 0
        i32.const -1
        i32.add
        local.set 0
        local.get 3
        i32.const -1
        i32.add
        local.tee 3
        br_if 0 (;@2;)
      end
    end
    local.get 1)
  (func $fibonacci (type 3) (param i32) (result i32)
    (local i32 i32 i32)
    block  ;; label = @1
      local.get 0
      i32.const 2
      i32.ge_s
      br_if 0 (;@1;)
      local.get 0
      i32.const 0
      i32.add
      return
    end
    i32.const 0
    local.set 1
    loop  ;; label = @1
      local.get 0
      i32.const -1
      i32.add
      call $fibonacci
      local.get 1
      i32.add
      local.set 1
      local.get 0
      i32.const 3
      i32.gt_u
      local.set 2
      local.get 0
      i32.const -2
      i32.add
      local.tee 3
      local.set 0
      local.get 2
      br_if 0 (;@1;)
    end
    local.get 3
    local.get 1
    i32.add)
  (func $get_counter (type 4) (result i32)
    i32.const 0
    i32.load offset=1049132)
  (func $increment (type 4) (result i32)
    (local i32)
    i32.const 0
    i32.const 0
    i32.load offset=1049132
    i32.const 1
    i32.add
    local.tee 0
    i32.store offset=1049132
    local.get 0)
  (func $is_prime (type 3) (param i32) (result i32)
    (local i32 i32 i32 i32)
    block  ;; label = @1
      block  ;; label = @2
        local.get 0
        i32.const 2
        i32.lt_s
        br_if 0 (;@2;)
        i32.const 1
        local.set 1
        local.get 0
        i32.const 4
        i32.lt_u
        br_if 1 (;@1;)
        local.get 0
        i32.const 3
        i32.rem_u
        local.set 2
        local.get 0
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 2
        i32.eqz
        br_if 0 (;@2;)
        i32.const 1
        local.set 1
        local.get 0
        i32.const 25
        i32.lt_u
        br_if 1 (;@1;)
        i32.const 5
        local.set 3
        i32.const 11
        local.set 2
        block  ;; label = @3
          block  ;; label = @4
            loop  ;; label = @5
              local.get 2
              i32.const 6
              i32.eq
              br_if 1 (;@4;)
              local.get 0
              local.get 3
              i32.rem_s
              i32.eqz
              br_if 3 (;@2;)
              local.get 3
              i32.const 2
              i32.add
              local.tee 4
              i32.eqz
              br_if 2 (;@3;)
              local.get 0
              local.get 4
              i32.rem_s
              i32.eqz
              br_if 3 (;@2;)
              local.get 3
              i32.const 6
              i32.add
              local.set 3
              local.get 2
              local.get 2
              i32.mul
              local.set 4
              local.get 2
              i32.const 6
              i32.add
              local.set 2
              local.get 4
              local.get 0
              i32.gt_s
              br_if 4 (;@1;)
              br 0 (;@5;)
            end
          end
          i32.const 1048732
          call $_ZN4core9panicking11panic_const23panic_const_rem_by_zero17h4d91c9c4a6b3b2e4E
          unreachable
        end
        i32.const 1048748
        call $_ZN4core9panicking11panic_const23panic_const_rem_by_zero17h4d91c9c4a6b3b2e4E
        unreachable
      end
      i32.const 0
      local.set 1
    end
    local.get 1)
  (func $reset_counter (type 5)
    i32.const 0
    i32.const 0
    i32.store offset=1049132)
  (func $_RNvCs1Y7DaGC1cwg_7___rustc12___rust_alloc (type 2) (param i32 i32) (result i32)
    local.get 0
    local.get 1
    call $_RNvCs1Y7DaGC1cwg_7___rustc11___rdl_alloc
    return)
  (func $_RNvCs1Y7DaGC1cwg_7___rustc14___rust_dealloc (type 6) (param i32 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    call $_RNvCs1Y7DaGC1cwg_7___rustc13___rdl_dealloc
    return)
  (func $_RNvCs1Y7DaGC1cwg_7___rustc14___rust_realloc (type 7) (param i32 i32 i32 i32) (result i32)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    call $_RNvCs1Y7DaGC1cwg_7___rustc13___rdl_realloc
    return)
  (func $_RNvCs1Y7DaGC1cwg_7___rustc35___rust_no_alloc_shim_is_unstable_v2 (type 5)
    return)
  (func $_RNvCs1Y7DaGC1cwg_7___rustc18___rust_start_panic (type 2) (param i32 i32) (result i32)
    call $_RNvCs1Y7DaGC1cwg_7___rustc12___rust_abort
    unreachable)
  (func $_RNvCs1Y7DaGC1cwg_7___rustc10rust_panic (type 0) (param i32 i32)
    local.get 0
    local.get 1
    call $_RNvCs1Y7DaGC1cwg_7___rustc18___rust_start_panic
    drop
    unreachable)
  (func $_RNvCs1Y7DaGC1cwg_7___rustc11___rdl_alloc (type 2) (param i32 i32) (result i32)
    block  ;; label = @1
      local.get 1
      i32.const 9
      i32.lt_u
      br_if 0 (;@1;)
      local.get 1
      local.get 0
      call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$8memalign17h4d574c3a3414c418E
      return
    end
    local.get 0
    call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$6malloc17he97e96981fab807eE)
  (func $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$8memalign17h4d574c3a3414c418E (type 2) (param i32 i32) (result i32)
    (local i32 i32 i32 i32 i32)
    i32.const 0
    local.set 2
    block  ;; label = @1
      local.get 1
      i32.const -65587
      local.get 0
      i32.const 16
      local.get 0
      i32.const 16
      i32.gt_u
      select
      local.tee 0
      i32.sub
      i32.ge_u
      br_if 0 (;@1;)
      local.get 0
      i32.const 16
      local.get 1
      i32.const 11
      i32.add
      i32.const -8
      i32.and
      local.get 1
      i32.const 11
      i32.lt_u
      select
      local.tee 3
      i32.add
      i32.const 12
      i32.add
      call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$6malloc17he97e96981fab807eE
      local.tee 1
      i32.eqz
      br_if 0 (;@1;)
      local.get 1
      i32.const -8
      i32.add
      local.set 2
      block  ;; label = @2
        block  ;; label = @3
          local.get 0
          i32.const -1
          i32.add
          local.tee 4
          local.get 1
          i32.and
          br_if 0 (;@3;)
          local.get 2
          local.set 0
          br 1 (;@2;)
        end
        local.get 1
        i32.const -4
        i32.add
        local.tee 5
        i32.load
        local.tee 6
        i32.const -8
        i32.and
        local.get 4
        local.get 1
        i32.add
        i32.const 0
        local.get 0
        i32.sub
        i32.and
        i32.const -8
        i32.add
        local.tee 1
        i32.const 0
        local.get 0
        local.get 1
        local.get 2
        i32.sub
        i32.const 16
        i32.gt_u
        select
        i32.add
        local.tee 0
        local.get 2
        i32.sub
        local.tee 1
        i32.sub
        local.set 4
        block  ;; label = @3
          local.get 6
          i32.const 3
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          local.get 4
          local.get 0
          i32.load offset=4
          i32.const 1
          i32.and
          i32.or
          i32.const 2
          i32.or
          i32.store offset=4
          local.get 0
          local.get 4
          i32.add
          local.tee 4
          local.get 4
          i32.load offset=4
          i32.const 1
          i32.or
          i32.store offset=4
          local.get 5
          local.get 1
          local.get 5
          i32.load
          i32.const 1
          i32.and
          i32.or
          i32.const 2
          i32.or
          i32.store
          local.get 2
          local.get 1
          i32.add
          local.tee 4
          local.get 4
          i32.load offset=4
          i32.const 1
          i32.or
          i32.store offset=4
          local.get 2
          local.get 1
          call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$13dispose_chunk17h0e84f108fd9f7b7cE
          br 1 (;@2;)
        end
        local.get 2
        i32.load
        local.set 2
        local.get 0
        local.get 4
        i32.store offset=4
        local.get 0
        local.get 2
        local.get 1
        i32.add
        i32.store
      end
      block  ;; label = @2
        local.get 0
        i32.load offset=4
        local.tee 1
        i32.const 3
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 1
        i32.const -8
        i32.and
        local.tee 2
        local.get 3
        i32.const 16
        i32.add
        i32.le_u
        br_if 0 (;@2;)
        local.get 0
        local.get 3
        local.get 1
        i32.const 1
        i32.and
        i32.or
        i32.const 2
        i32.or
        i32.store offset=4
        local.get 0
        local.get 3
        i32.add
        local.tee 1
        local.get 2
        local.get 3
        i32.sub
        local.tee 3
        i32.const 3
        i32.or
        i32.store offset=4
        local.get 0
        local.get 2
        i32.add
        local.tee 2
        local.get 2
        i32.load offset=4
        i32.const 1
        i32.or
        i32.store offset=4
        local.get 1
        local.get 3
        call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$13dispose_chunk17h0e84f108fd9f7b7cE
      end
      local.get 0
      i32.const 8
      i32.add
      local.set 2
    end
    local.get 2)
  (func $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$6malloc17he97e96981fab807eE (type 3) (param i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i64)
    global.get $__stack_pointer
    i32.const 16
    i32.sub
    local.tee 1
    global.set $__stack_pointer
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                local.get 0
                i32.const 245
                i32.lt_u
                br_if 0 (;@6;)
                block  ;; label = @7
                  local.get 0
                  i32.const -65588
                  i32.le_u
                  br_if 0 (;@7;)
                  i32.const 0
                  local.set 0
                  br 6 (;@1;)
                end
                local.get 0
                i32.const 11
                i32.add
                local.tee 2
                i32.const -8
                i32.and
                local.set 3
                i32.const 0
                i32.load offset=1049548
                local.tee 4
                i32.eqz
                br_if 4 (;@2;)
                i32.const 31
                local.set 5
                block  ;; label = @7
                  local.get 0
                  i32.const 16777204
                  i32.gt_u
                  br_if 0 (;@7;)
                  local.get 3
                  i32.const 38
                  local.get 2
                  i32.const 8
                  i32.shr_u
                  i32.clz
                  local.tee 0
                  i32.sub
                  i32.shr_u
                  i32.const 1
                  i32.and
                  local.get 0
                  i32.const 1
                  i32.shl
                  i32.sub
                  i32.const 62
                  i32.add
                  local.set 5
                end
                i32.const 0
                local.get 3
                i32.sub
                local.set 2
                block  ;; label = @7
                  local.get 5
                  i32.const 2
                  i32.shl
                  i32.const 1049136
                  i32.add
                  i32.load
                  local.tee 6
                  br_if 0 (;@7;)
                  i32.const 0
                  local.set 7
                  i32.const 0
                  local.set 0
                  br 2 (;@5;)
                end
                i32.const 0
                local.set 7
                local.get 3
                i32.const 0
                i32.const 25
                local.get 5
                i32.const 1
                i32.shr_u
                i32.sub
                local.get 5
                i32.const 31
                i32.eq
                select
                i32.shl
                local.set 8
                i32.const 0
                local.set 0
                loop  ;; label = @7
                  block  ;; label = @8
                    local.get 6
                    local.tee 6
                    i32.load offset=4
                    i32.const -8
                    i32.and
                    local.tee 9
                    local.get 3
                    i32.lt_u
                    br_if 0 (;@8;)
                    local.get 9
                    local.get 3
                    i32.sub
                    local.tee 9
                    local.get 2
                    i32.ge_u
                    br_if 0 (;@8;)
                    local.get 6
                    local.set 7
                    local.get 9
                    local.set 2
                    local.get 9
                    br_if 0 (;@8;)
                    i32.const 0
                    local.set 2
                    local.get 6
                    local.set 0
                    local.get 6
                    local.set 7
                    br 4 (;@4;)
                  end
                  local.get 6
                  i32.load offset=20
                  local.tee 9
                  local.get 0
                  local.get 9
                  local.get 6
                  local.get 8
                  i32.const 29
                  i32.shr_u
                  i32.const 4
                  i32.and
                  i32.add
                  i32.load offset=16
                  local.tee 6
                  i32.ne
                  select
                  local.get 0
                  local.get 9
                  select
                  local.set 0
                  local.get 8
                  i32.const 1
                  i32.shl
                  local.set 8
                  local.get 6
                  i32.eqz
                  br_if 2 (;@5;)
                  br 0 (;@7;)
                end
              end
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      block  ;; label = @10
                        block  ;; label = @11
                          i32.const 0
                          i32.load offset=1049544
                          local.tee 6
                          i32.const 16
                          local.get 0
                          i32.const 11
                          i32.add
                          i32.const 504
                          i32.and
                          local.get 0
                          i32.const 11
                          i32.lt_u
                          select
                          local.tee 3
                          i32.const 3
                          i32.shr_u
                          local.tee 2
                          i32.shr_u
                          local.tee 0
                          i32.const 3
                          i32.and
                          i32.eqz
                          br_if 0 (;@11;)
                          local.get 0
                          i32.const -1
                          i32.xor
                          i32.const 1
                          i32.and
                          local.get 2
                          i32.add
                          local.tee 8
                          i32.const 3
                          i32.shl
                          local.tee 3
                          i32.const 1049280
                          i32.add
                          local.tee 0
                          local.get 3
                          i32.const 1049288
                          i32.add
                          i32.load
                          local.tee 2
                          i32.load offset=8
                          local.tee 7
                          i32.eq
                          br_if 1 (;@10;)
                          local.get 7
                          local.get 0
                          i32.store offset=12
                          local.get 0
                          local.get 7
                          i32.store offset=8
                          br 2 (;@9;)
                        end
                        local.get 3
                        i32.const 0
                        i32.load offset=1049552
                        i32.le_u
                        br_if 8 (;@2;)
                        local.get 0
                        br_if 2 (;@8;)
                        i32.const 0
                        i32.load offset=1049548
                        local.tee 0
                        i32.eqz
                        br_if 8 (;@2;)
                        local.get 0
                        i32.ctz
                        i32.const 2
                        i32.shl
                        i32.const 1049136
                        i32.add
                        i32.load
                        local.tee 6
                        i32.load offset=4
                        i32.const -8
                        i32.and
                        local.get 3
                        i32.sub
                        local.set 2
                        local.get 6
                        local.set 7
                        loop  ;; label = @11
                          block  ;; label = @12
                            local.get 7
                            i32.load offset=16
                            local.tee 0
                            br_if 0 (;@12;)
                            local.get 7
                            i32.load offset=20
                            local.tee 0
                            br_if 0 (;@12;)
                            local.get 6
                            i32.load offset=24
                            local.set 5
                            block  ;; label = @13
                              block  ;; label = @14
                                block  ;; label = @15
                                  local.get 6
                                  i32.load offset=12
                                  local.tee 0
                                  local.get 6
                                  i32.ne
                                  br_if 0 (;@15;)
                                  local.get 6
                                  i32.const 20
                                  i32.const 16
                                  local.get 6
                                  i32.load offset=20
                                  local.tee 0
                                  select
                                  i32.add
                                  i32.load
                                  local.tee 7
                                  br_if 1 (;@14;)
                                  i32.const 0
                                  local.set 0
                                  br 2 (;@13;)
                                end
                                local.get 6
                                i32.load offset=8
                                local.tee 7
                                local.get 0
                                i32.store offset=12
                                local.get 0
                                local.get 7
                                i32.store offset=8
                                br 1 (;@13;)
                              end
                              local.get 6
                              i32.const 20
                              i32.add
                              local.get 6
                              i32.const 16
                              i32.add
                              local.get 0
                              select
                              local.set 8
                              loop  ;; label = @14
                                local.get 8
                                local.set 9
                                local.get 7
                                local.tee 0
                                i32.const 20
                                i32.add
                                local.get 0
                                i32.const 16
                                i32.add
                                local.get 0
                                i32.load offset=20
                                local.tee 7
                                select
                                local.set 8
                                local.get 0
                                i32.const 20
                                i32.const 16
                                local.get 7
                                select
                                i32.add
                                i32.load
                                local.tee 7
                                br_if 0 (;@14;)
                              end
                              local.get 9
                              i32.const 0
                              i32.store
                            end
                            local.get 5
                            i32.eqz
                            br_if 6 (;@6;)
                            block  ;; label = @13
                              block  ;; label = @14
                                local.get 6
                                local.get 6
                                i32.load offset=28
                                i32.const 2
                                i32.shl
                                i32.const 1049136
                                i32.add
                                local.tee 7
                                i32.load
                                i32.eq
                                br_if 0 (;@14;)
                                block  ;; label = @15
                                  local.get 5
                                  i32.load offset=16
                                  local.get 6
                                  i32.eq
                                  br_if 0 (;@15;)
                                  local.get 5
                                  local.get 0
                                  i32.store offset=20
                                  local.get 0
                                  br_if 2 (;@13;)
                                  br 9 (;@6;)
                                end
                                local.get 5
                                local.get 0
                                i32.store offset=16
                                local.get 0
                                br_if 1 (;@13;)
                                br 8 (;@6;)
                              end
                              local.get 7
                              local.get 0
                              i32.store
                              local.get 0
                              i32.eqz
                              br_if 6 (;@7;)
                            end
                            local.get 0
                            local.get 5
                            i32.store offset=24
                            block  ;; label = @13
                              local.get 6
                              i32.load offset=16
                              local.tee 7
                              i32.eqz
                              br_if 0 (;@13;)
                              local.get 0
                              local.get 7
                              i32.store offset=16
                              local.get 7
                              local.get 0
                              i32.store offset=24
                            end
                            local.get 6
                            i32.load offset=20
                            local.tee 7
                            i32.eqz
                            br_if 6 (;@6;)
                            local.get 0
                            local.get 7
                            i32.store offset=20
                            local.get 7
                            local.get 0
                            i32.store offset=24
                            br 6 (;@6;)
                          end
                          local.get 0
                          i32.load offset=4
                          i32.const -8
                          i32.and
                          local.get 3
                          i32.sub
                          local.tee 7
                          local.get 2
                          local.get 7
                          local.get 2
                          i32.lt_u
                          local.tee 7
                          select
                          local.set 2
                          local.get 0
                          local.get 6
                          local.get 7
                          select
                          local.set 6
                          local.get 0
                          local.set 7
                          br 0 (;@11;)
                        end
                      end
                      i32.const 0
                      local.get 6
                      i32.const -2
                      local.get 8
                      i32.rotl
                      i32.and
                      i32.store offset=1049544
                    end
                    local.get 2
                    i32.const 8
                    i32.add
                    local.set 0
                    local.get 2
                    local.get 3
                    i32.const 3
                    i32.or
                    i32.store offset=4
                    local.get 2
                    local.get 3
                    i32.add
                    local.tee 3
                    local.get 3
                    i32.load offset=4
                    i32.const 1
                    i32.or
                    i32.store offset=4
                    br 7 (;@1;)
                  end
                  block  ;; label = @8
                    block  ;; label = @9
                      local.get 0
                      local.get 2
                      i32.shl
                      i32.const 2
                      local.get 2
                      i32.shl
                      local.tee 0
                      i32.const 0
                      local.get 0
                      i32.sub
                      i32.or
                      i32.and
                      i32.ctz
                      local.tee 9
                      i32.const 3
                      i32.shl
                      local.tee 2
                      i32.const 1049280
                      i32.add
                      local.tee 7
                      local.get 2
                      i32.const 1049288
                      i32.add
                      i32.load
                      local.tee 0
                      i32.load offset=8
                      local.tee 8
                      i32.eq
                      br_if 0 (;@9;)
                      local.get 8
                      local.get 7
                      i32.store offset=12
                      local.get 7
                      local.get 8
                      i32.store offset=8
                      br 1 (;@8;)
                    end
                    i32.const 0
                    local.get 6
                    i32.const -2
                    local.get 9
                    i32.rotl
                    i32.and
                    i32.store offset=1049544
                  end
                  local.get 0
                  local.get 3
                  i32.const 3
                  i32.or
                  i32.store offset=4
                  local.get 0
                  local.get 3
                  i32.add
                  local.tee 6
                  local.get 2
                  local.get 3
                  i32.sub
                  local.tee 7
                  i32.const 1
                  i32.or
                  i32.store offset=4
                  local.get 0
                  local.get 2
                  i32.add
                  local.get 7
                  i32.store
                  block  ;; label = @8
                    i32.const 0
                    i32.load offset=1049552
                    local.tee 2
                    i32.eqz
                    br_if 0 (;@8;)
                    i32.const 0
                    i32.load offset=1049560
                    local.set 3
                    block  ;; label = @9
                      block  ;; label = @10
                        i32.const 0
                        i32.load offset=1049544
                        local.tee 8
                        i32.const 1
                        local.get 2
                        i32.const 3
                        i32.shr_u
                        i32.shl
                        local.tee 9
                        i32.and
                        br_if 0 (;@10;)
                        i32.const 0
                        local.get 8
                        local.get 9
                        i32.or
                        i32.store offset=1049544
                        local.get 2
                        i32.const -8
                        i32.and
                        i32.const 1049280
                        i32.add
                        local.tee 2
                        local.set 8
                        br 1 (;@9;)
                      end
                      local.get 2
                      i32.const -8
                      i32.and
                      local.tee 2
                      i32.const 1049280
                      i32.add
                      local.set 8
                      local.get 2
                      i32.const 1049288
                      i32.add
                      i32.load
                      local.set 2
                    end
                    local.get 8
                    local.get 3
                    i32.store offset=8
                    local.get 2
                    local.get 3
                    i32.store offset=12
                    local.get 3
                    local.get 8
                    i32.store offset=12
                    local.get 3
                    local.get 2
                    i32.store offset=8
                  end
                  local.get 0
                  i32.const 8
                  i32.add
                  local.set 0
                  i32.const 0
                  local.get 6
                  i32.store offset=1049560
                  i32.const 0
                  local.get 7
                  i32.store offset=1049552
                  br 6 (;@1;)
                end
                i32.const 0
                i32.const 0
                i32.load offset=1049548
                i32.const -2
                local.get 6
                i32.load offset=28
                i32.rotl
                i32.and
                i32.store offset=1049548
              end
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    local.get 2
                    i32.const 16
                    i32.lt_u
                    br_if 0 (;@8;)
                    local.get 6
                    local.get 3
                    i32.const 3
                    i32.or
                    i32.store offset=4
                    local.get 6
                    local.get 3
                    i32.add
                    local.tee 7
                    local.get 2
                    i32.const 1
                    i32.or
                    i32.store offset=4
                    local.get 7
                    local.get 2
                    i32.add
                    local.get 2
                    i32.store
                    i32.const 0
                    i32.load offset=1049552
                    local.tee 8
                    i32.eqz
                    br_if 1 (;@7;)
                    i32.const 0
                    i32.load offset=1049560
                    local.set 0
                    block  ;; label = @9
                      block  ;; label = @10
                        i32.const 0
                        i32.load offset=1049544
                        local.tee 9
                        i32.const 1
                        local.get 8
                        i32.const 3
                        i32.shr_u
                        i32.shl
                        local.tee 5
                        i32.and
                        br_if 0 (;@10;)
                        i32.const 0
                        local.get 9
                        local.get 5
                        i32.or
                        i32.store offset=1049544
                        local.get 8
                        i32.const -8
                        i32.and
                        i32.const 1049280
                        i32.add
                        local.tee 8
                        local.set 9
                        br 1 (;@9;)
                      end
                      local.get 8
                      i32.const -8
                      i32.and
                      local.tee 8
                      i32.const 1049280
                      i32.add
                      local.set 9
                      local.get 8
                      i32.const 1049288
                      i32.add
                      i32.load
                      local.set 8
                    end
                    local.get 9
                    local.get 0
                    i32.store offset=8
                    local.get 8
                    local.get 0
                    i32.store offset=12
                    local.get 0
                    local.get 9
                    i32.store offset=12
                    local.get 0
                    local.get 8
                    i32.store offset=8
                    br 1 (;@7;)
                  end
                  local.get 6
                  local.get 2
                  local.get 3
                  i32.add
                  local.tee 0
                  i32.const 3
                  i32.or
                  i32.store offset=4
                  local.get 6
                  local.get 0
                  i32.add
                  local.tee 0
                  local.get 0
                  i32.load offset=4
                  i32.const 1
                  i32.or
                  i32.store offset=4
                  br 1 (;@6;)
                end
                i32.const 0
                local.get 7
                i32.store offset=1049560
                i32.const 0
                local.get 2
                i32.store offset=1049552
              end
              local.get 6
              i32.const 8
              i32.add
              local.tee 0
              i32.eqz
              br_if 3 (;@2;)
              br 4 (;@1;)
            end
            block  ;; label = @5
              local.get 0
              local.get 7
              i32.or
              br_if 0 (;@5;)
              i32.const 0
              local.set 7
              i32.const 2
              local.get 5
              i32.shl
              local.tee 0
              i32.const 0
              local.get 0
              i32.sub
              i32.or
              local.get 4
              i32.and
              local.tee 0
              i32.eqz
              br_if 3 (;@2;)
              local.get 0
              i32.ctz
              i32.const 2
              i32.shl
              i32.const 1049136
              i32.add
              i32.load
              local.set 0
            end
            local.get 0
            i32.eqz
            br_if 1 (;@3;)
          end
          loop  ;; label = @4
            local.get 0
            i32.load offset=4
            i32.const -8
            i32.and
            local.tee 6
            local.get 3
            i32.sub
            local.tee 8
            local.get 2
            local.get 8
            local.get 2
            i32.lt_u
            local.tee 9
            select
            local.set 5
            local.get 6
            local.get 3
            i32.lt_u
            local.set 8
            local.get 0
            local.get 7
            local.get 9
            select
            local.set 9
            block  ;; label = @5
              local.get 0
              i32.load offset=16
              local.tee 6
              br_if 0 (;@5;)
              local.get 0
              i32.load offset=20
              local.set 6
            end
            local.get 2
            local.get 5
            local.get 8
            select
            local.set 2
            local.get 7
            local.get 9
            local.get 8
            select
            local.set 7
            local.get 6
            local.set 0
            local.get 6
            br_if 0 (;@4;)
          end
        end
        local.get 7
        i32.eqz
        br_if 0 (;@2;)
        block  ;; label = @3
          i32.const 0
          i32.load offset=1049552
          local.tee 0
          local.get 3
          i32.lt_u
          br_if 0 (;@3;)
          local.get 2
          local.get 0
          local.get 3
          i32.sub
          i32.ge_u
          br_if 1 (;@2;)
        end
        local.get 7
        i32.load offset=24
        local.set 5
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 7
              i32.load offset=12
              local.tee 0
              local.get 7
              i32.ne
              br_if 0 (;@5;)
              local.get 7
              i32.const 20
              i32.const 16
              local.get 7
              i32.load offset=20
              local.tee 0
              select
              i32.add
              i32.load
              local.tee 6
              br_if 1 (;@4;)
              i32.const 0
              local.set 0
              br 2 (;@3;)
            end
            local.get 7
            i32.load offset=8
            local.tee 6
            local.get 0
            i32.store offset=12
            local.get 0
            local.get 6
            i32.store offset=8
            br 1 (;@3;)
          end
          local.get 7
          i32.const 20
          i32.add
          local.get 7
          i32.const 16
          i32.add
          local.get 0
          select
          local.set 8
          loop  ;; label = @4
            local.get 8
            local.set 9
            local.get 6
            local.tee 0
            i32.const 20
            i32.add
            local.get 0
            i32.const 16
            i32.add
            local.get 0
            i32.load offset=20
            local.tee 6
            select
            local.set 8
            local.get 0
            i32.const 20
            i32.const 16
            local.get 6
            select
            i32.add
            i32.load
            local.tee 6
            br_if 0 (;@4;)
          end
          local.get 9
          i32.const 0
          i32.store
        end
        block  ;; label = @3
          local.get 5
          i32.eqz
          br_if 0 (;@3;)
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                local.get 7
                local.get 7
                i32.load offset=28
                i32.const 2
                i32.shl
                i32.const 1049136
                i32.add
                local.tee 6
                i32.load
                i32.eq
                br_if 0 (;@6;)
                block  ;; label = @7
                  local.get 5
                  i32.load offset=16
                  local.get 7
                  i32.eq
                  br_if 0 (;@7;)
                  local.get 5
                  local.get 0
                  i32.store offset=20
                  local.get 0
                  br_if 2 (;@5;)
                  br 4 (;@3;)
                end
                local.get 5
                local.get 0
                i32.store offset=16
                local.get 0
                br_if 1 (;@5;)
                br 3 (;@3;)
              end
              local.get 6
              local.get 0
              i32.store
              local.get 0
              i32.eqz
              br_if 1 (;@4;)
            end
            local.get 0
            local.get 5
            i32.store offset=24
            block  ;; label = @5
              local.get 7
              i32.load offset=16
              local.tee 6
              i32.eqz
              br_if 0 (;@5;)
              local.get 0
              local.get 6
              i32.store offset=16
              local.get 6
              local.get 0
              i32.store offset=24
            end
            local.get 7
            i32.load offset=20
            local.tee 6
            i32.eqz
            br_if 1 (;@3;)
            local.get 0
            local.get 6
            i32.store offset=20
            local.get 6
            local.get 0
            i32.store offset=24
            br 1 (;@3;)
          end
          i32.const 0
          i32.const 0
          i32.load offset=1049548
          i32.const -2
          local.get 7
          i32.load offset=28
          i32.rotl
          i32.and
          i32.store offset=1049548
        end
        block  ;; label = @3
          block  ;; label = @4
            local.get 2
            i32.const 16
            i32.lt_u
            br_if 0 (;@4;)
            local.get 7
            local.get 3
            i32.const 3
            i32.or
            i32.store offset=4
            local.get 7
            local.get 3
            i32.add
            local.tee 0
            local.get 2
            i32.const 1
            i32.or
            i32.store offset=4
            local.get 0
            local.get 2
            i32.add
            local.get 2
            i32.store
            block  ;; label = @5
              local.get 2
              i32.const 256
              i32.lt_u
              br_if 0 (;@5;)
              local.get 0
              local.get 2
              call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$18insert_large_chunk17hc7cb11ca1345ed38E
              br 2 (;@3;)
            end
            block  ;; label = @5
              block  ;; label = @6
                i32.const 0
                i32.load offset=1049544
                local.tee 6
                i32.const 1
                local.get 2
                i32.const 3
                i32.shr_u
                i32.shl
                local.tee 8
                i32.and
                br_if 0 (;@6;)
                i32.const 0
                local.get 6
                local.get 8
                i32.or
                i32.store offset=1049544
                local.get 2
                i32.const 248
                i32.and
                i32.const 1049280
                i32.add
                local.tee 2
                local.set 6
                br 1 (;@5;)
              end
              local.get 2
              i32.const 248
              i32.and
              local.tee 2
              i32.const 1049280
              i32.add
              local.set 6
              local.get 2
              i32.const 1049288
              i32.add
              i32.load
              local.set 2
            end
            local.get 6
            local.get 0
            i32.store offset=8
            local.get 2
            local.get 0
            i32.store offset=12
            local.get 0
            local.get 6
            i32.store offset=12
            local.get 0
            local.get 2
            i32.store offset=8
            br 1 (;@3;)
          end
          local.get 7
          local.get 2
          local.get 3
          i32.add
          local.tee 0
          i32.const 3
          i32.or
          i32.store offset=4
          local.get 7
          local.get 0
          i32.add
          local.tee 0
          local.get 0
          i32.load offset=4
          i32.const 1
          i32.or
          i32.store offset=4
        end
        local.get 7
        i32.const 8
        i32.add
        local.tee 0
        br_if 1 (;@1;)
      end
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  i32.const 0
                  i32.load offset=1049552
                  local.tee 0
                  local.get 3
                  i32.ge_u
                  br_if 0 (;@7;)
                  block  ;; label = @8
                    i32.const 0
                    i32.load offset=1049556
                    local.tee 0
                    local.get 3
                    i32.gt_u
                    br_if 0 (;@8;)
                    local.get 1
                    i32.const 4
                    i32.add
                    i32.const 1049588
                    local.get 3
                    i32.const 65583
                    i32.add
                    i32.const -65536
                    i32.and
                    call $_ZN61_$LT$dlmalloc..sys..System$u20$as$u20$dlmalloc..Allocator$GT$5alloc17h5d11e6618597802fE
                    block  ;; label = @9
                      local.get 1
                      i32.load offset=4
                      local.tee 6
                      br_if 0 (;@9;)
                      i32.const 0
                      local.set 0
                      br 8 (;@1;)
                    end
                    local.get 1
                    i32.load offset=12
                    local.set 5
                    i32.const 0
                    i32.const 0
                    i32.load offset=1049568
                    local.get 1
                    i32.load offset=8
                    local.tee 9
                    i32.add
                    local.tee 0
                    i32.store offset=1049568
                    i32.const 0
                    local.get 0
                    i32.const 0
                    i32.load offset=1049572
                    local.tee 2
                    local.get 0
                    local.get 2
                    i32.gt_u
                    select
                    i32.store offset=1049572
                    block  ;; label = @9
                      block  ;; label = @10
                        block  ;; label = @11
                          i32.const 0
                          i32.load offset=1049564
                          local.tee 2
                          i32.eqz
                          br_if 0 (;@11;)
                          i32.const 1049264
                          local.set 0
                          loop  ;; label = @12
                            local.get 6
                            local.get 0
                            i32.load
                            local.tee 7
                            local.get 0
                            i32.load offset=4
                            local.tee 8
                            i32.add
                            i32.eq
                            br_if 2 (;@10;)
                            local.get 0
                            i32.load offset=8
                            local.tee 0
                            br_if 0 (;@12;)
                            br 3 (;@9;)
                          end
                        end
                        block  ;; label = @11
                          block  ;; label = @12
                            i32.const 0
                            i32.load offset=1049580
                            local.tee 0
                            i32.eqz
                            br_if 0 (;@12;)
                            local.get 6
                            local.get 0
                            i32.ge_u
                            br_if 1 (;@11;)
                          end
                          i32.const 0
                          local.get 6
                          i32.store offset=1049580
                        end
                        i32.const 0
                        i32.const 4095
                        i32.store offset=1049584
                        i32.const 0
                        local.get 5
                        i32.store offset=1049276
                        i32.const 0
                        local.get 9
                        i32.store offset=1049268
                        i32.const 0
                        local.get 6
                        i32.store offset=1049264
                        i32.const 0
                        i32.const 1049280
                        i32.store offset=1049292
                        i32.const 0
                        i32.const 1049288
                        i32.store offset=1049300
                        i32.const 0
                        i32.const 1049280
                        i32.store offset=1049288
                        i32.const 0
                        i32.const 1049296
                        i32.store offset=1049308
                        i32.const 0
                        i32.const 1049288
                        i32.store offset=1049296
                        i32.const 0
                        i32.const 1049304
                        i32.store offset=1049316
                        i32.const 0
                        i32.const 1049296
                        i32.store offset=1049304
                        i32.const 0
                        i32.const 1049312
                        i32.store offset=1049324
                        i32.const 0
                        i32.const 1049304
                        i32.store offset=1049312
                        i32.const 0
                        i32.const 1049320
                        i32.store offset=1049332
                        i32.const 0
                        i32.const 1049312
                        i32.store offset=1049320
                        i32.const 0
                        i32.const 1049328
                        i32.store offset=1049340
                        i32.const 0
                        i32.const 1049320
                        i32.store offset=1049328
                        i32.const 0
                        i32.const 1049336
                        i32.store offset=1049348
                        i32.const 0
                        i32.const 1049328
                        i32.store offset=1049336
                        i32.const 0
                        i32.const 1049344
                        i32.store offset=1049356
                        i32.const 0
                        i32.const 1049336
                        i32.store offset=1049344
                        i32.const 0
                        i32.const 1049344
                        i32.store offset=1049352
                        i32.const 0
                        i32.const 1049352
                        i32.store offset=1049364
                        i32.const 0
                        i32.const 1049352
                        i32.store offset=1049360
                        i32.const 0
                        i32.const 1049360
                        i32.store offset=1049372
                        i32.const 0
                        i32.const 1049360
                        i32.store offset=1049368
                        i32.const 0
                        i32.const 1049368
                        i32.store offset=1049380
                        i32.const 0
                        i32.const 1049368
                        i32.store offset=1049376
                        i32.const 0
                        i32.const 1049376
                        i32.store offset=1049388
                        i32.const 0
                        i32.const 1049376
                        i32.store offset=1049384
                        i32.const 0
                        i32.const 1049384
                        i32.store offset=1049396
                        i32.const 0
                        i32.const 1049384
                        i32.store offset=1049392
                        i32.const 0
                        i32.const 1049392
                        i32.store offset=1049404
                        i32.const 0
                        i32.const 1049392
                        i32.store offset=1049400
                        i32.const 0
                        i32.const 1049400
                        i32.store offset=1049412
                        i32.const 0
                        i32.const 1049400
                        i32.store offset=1049408
                        i32.const 0
                        i32.const 1049408
                        i32.store offset=1049420
                        i32.const 0
                        i32.const 1049416
                        i32.store offset=1049428
                        i32.const 0
                        i32.const 1049408
                        i32.store offset=1049416
                        i32.const 0
                        i32.const 1049424
                        i32.store offset=1049436
                        i32.const 0
                        i32.const 1049416
                        i32.store offset=1049424
                        i32.const 0
                        i32.const 1049432
                        i32.store offset=1049444
                        i32.const 0
                        i32.const 1049424
                        i32.store offset=1049432
                        i32.const 0
                        i32.const 1049440
                        i32.store offset=1049452
                        i32.const 0
                        i32.const 1049432
                        i32.store offset=1049440
                        i32.const 0
                        i32.const 1049448
                        i32.store offset=1049460
                        i32.const 0
                        i32.const 1049440
                        i32.store offset=1049448
                        i32.const 0
                        i32.const 1049456
                        i32.store offset=1049468
                        i32.const 0
                        i32.const 1049448
                        i32.store offset=1049456
                        i32.const 0
                        i32.const 1049464
                        i32.store offset=1049476
                        i32.const 0
                        i32.const 1049456
                        i32.store offset=1049464
                        i32.const 0
                        i32.const 1049472
                        i32.store offset=1049484
                        i32.const 0
                        i32.const 1049464
                        i32.store offset=1049472
                        i32.const 0
                        i32.const 1049480
                        i32.store offset=1049492
                        i32.const 0
                        i32.const 1049472
                        i32.store offset=1049480
                        i32.const 0
                        i32.const 1049488
                        i32.store offset=1049500
                        i32.const 0
                        i32.const 1049480
                        i32.store offset=1049488
                        i32.const 0
                        i32.const 1049496
                        i32.store offset=1049508
                        i32.const 0
                        i32.const 1049488
                        i32.store offset=1049496
                        i32.const 0
                        i32.const 1049504
                        i32.store offset=1049516
                        i32.const 0
                        i32.const 1049496
                        i32.store offset=1049504
                        i32.const 0
                        i32.const 1049512
                        i32.store offset=1049524
                        i32.const 0
                        i32.const 1049504
                        i32.store offset=1049512
                        i32.const 0
                        i32.const 1049520
                        i32.store offset=1049532
                        i32.const 0
                        i32.const 1049512
                        i32.store offset=1049520
                        i32.const 0
                        i32.const 1049528
                        i32.store offset=1049540
                        i32.const 0
                        i32.const 1049520
                        i32.store offset=1049528
                        i32.const 0
                        local.get 6
                        i32.const 15
                        i32.add
                        i32.const -8
                        i32.and
                        local.tee 0
                        i32.const -8
                        i32.add
                        local.tee 2
                        i32.store offset=1049564
                        i32.const 0
                        i32.const 1049528
                        i32.store offset=1049536
                        i32.const 0
                        local.get 6
                        local.get 0
                        i32.sub
                        local.get 9
                        i32.const -40
                        i32.add
                        local.tee 0
                        i32.add
                        i32.const 8
                        i32.add
                        local.tee 7
                        i32.store offset=1049556
                        local.get 2
                        local.get 7
                        i32.const 1
                        i32.or
                        i32.store offset=4
                        local.get 6
                        local.get 0
                        i32.add
                        i32.const 40
                        i32.store offset=4
                        i32.const 0
                        i32.const 2097152
                        i32.store offset=1049576
                        br 8 (;@2;)
                      end
                      local.get 2
                      local.get 6
                      i32.ge_u
                      br_if 0 (;@9;)
                      local.get 7
                      local.get 2
                      i32.gt_u
                      br_if 0 (;@9;)
                      local.get 0
                      i32.load offset=12
                      local.tee 7
                      i32.const 1
                      i32.and
                      br_if 0 (;@9;)
                      local.get 7
                      i32.const 1
                      i32.shr_u
                      local.get 5
                      i32.eq
                      br_if 3 (;@6;)
                    end
                    i32.const 0
                    i32.const 0
                    i32.load offset=1049580
                    local.tee 0
                    local.get 6
                    local.get 0
                    local.get 6
                    i32.lt_u
                    select
                    i32.store offset=1049580
                    local.get 6
                    local.get 9
                    i32.add
                    local.set 7
                    i32.const 1049264
                    local.set 0
                    block  ;; label = @9
                      block  ;; label = @10
                        block  ;; label = @11
                          loop  ;; label = @12
                            local.get 0
                            i32.load
                            local.tee 8
                            local.get 7
                            i32.eq
                            br_if 1 (;@11;)
                            local.get 0
                            i32.load offset=8
                            local.tee 0
                            br_if 0 (;@12;)
                            br 2 (;@10;)
                          end
                        end
                        local.get 0
                        i32.load offset=12
                        local.tee 7
                        i32.const 1
                        i32.and
                        br_if 0 (;@10;)
                        local.get 7
                        i32.const 1
                        i32.shr_u
                        local.get 5
                        i32.eq
                        br_if 1 (;@9;)
                      end
                      i32.const 1049264
                      local.set 0
                      block  ;; label = @10
                        loop  ;; label = @11
                          block  ;; label = @12
                            local.get 0
                            i32.load
                            local.tee 7
                            local.get 2
                            i32.gt_u
                            br_if 0 (;@12;)
                            local.get 2
                            local.get 7
                            local.get 0
                            i32.load offset=4
                            i32.add
                            local.tee 7
                            i32.lt_u
                            br_if 2 (;@10;)
                          end
                          local.get 0
                          i32.load offset=8
                          local.set 0
                          br 0 (;@11;)
                        end
                      end
                      i32.const 0
                      local.get 6
                      i32.const 15
                      i32.add
                      i32.const -8
                      i32.and
                      local.tee 0
                      i32.const -8
                      i32.add
                      local.tee 8
                      i32.store offset=1049564
                      i32.const 0
                      local.get 6
                      local.get 0
                      i32.sub
                      local.get 9
                      i32.const -40
                      i32.add
                      local.tee 0
                      i32.add
                      i32.const 8
                      i32.add
                      local.tee 4
                      i32.store offset=1049556
                      local.get 8
                      local.get 4
                      i32.const 1
                      i32.or
                      i32.store offset=4
                      local.get 6
                      local.get 0
                      i32.add
                      i32.const 40
                      i32.store offset=4
                      i32.const 0
                      i32.const 2097152
                      i32.store offset=1049576
                      local.get 2
                      local.get 7
                      i32.const -32
                      i32.add
                      i32.const -8
                      i32.and
                      i32.const -8
                      i32.add
                      local.tee 0
                      local.get 0
                      local.get 2
                      i32.const 16
                      i32.add
                      i32.lt_u
                      select
                      local.tee 8
                      i32.const 27
                      i32.store offset=4
                      i32.const 0
                      i64.load offset=1049264 align=4
                      local.set 10
                      local.get 8
                      i32.const 16
                      i32.add
                      i32.const 0
                      i64.load offset=1049272 align=4
                      i64.store align=4
                      local.get 8
                      i32.const 8
                      i32.add
                      local.tee 0
                      local.get 10
                      i64.store align=4
                      i32.const 0
                      local.get 5
                      i32.store offset=1049276
                      i32.const 0
                      local.get 9
                      i32.store offset=1049268
                      i32.const 0
                      local.get 6
                      i32.store offset=1049264
                      i32.const 0
                      local.get 0
                      i32.store offset=1049272
                      local.get 8
                      i32.const 28
                      i32.add
                      local.set 0
                      loop  ;; label = @10
                        local.get 0
                        i32.const 7
                        i32.store
                        local.get 0
                        i32.const 4
                        i32.add
                        local.tee 0
                        local.get 7
                        i32.lt_u
                        br_if 0 (;@10;)
                      end
                      local.get 8
                      local.get 2
                      i32.eq
                      br_if 7 (;@2;)
                      local.get 8
                      local.get 8
                      i32.load offset=4
                      i32.const -2
                      i32.and
                      i32.store offset=4
                      local.get 2
                      local.get 8
                      local.get 2
                      i32.sub
                      local.tee 0
                      i32.const 1
                      i32.or
                      i32.store offset=4
                      local.get 8
                      local.get 0
                      i32.store
                      block  ;; label = @10
                        local.get 0
                        i32.const 256
                        i32.lt_u
                        br_if 0 (;@10;)
                        local.get 2
                        local.get 0
                        call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$18insert_large_chunk17hc7cb11ca1345ed38E
                        br 8 (;@2;)
                      end
                      block  ;; label = @10
                        block  ;; label = @11
                          i32.const 0
                          i32.load offset=1049544
                          local.tee 7
                          i32.const 1
                          local.get 0
                          i32.const 3
                          i32.shr_u
                          i32.shl
                          local.tee 6
                          i32.and
                          br_if 0 (;@11;)
                          i32.const 0
                          local.get 7
                          local.get 6
                          i32.or
                          i32.store offset=1049544
                          local.get 0
                          i32.const 248
                          i32.and
                          i32.const 1049280
                          i32.add
                          local.tee 0
                          local.set 7
                          br 1 (;@10;)
                        end
                        local.get 0
                        i32.const 248
                        i32.and
                        local.tee 0
                        i32.const 1049280
                        i32.add
                        local.set 7
                        local.get 0
                        i32.const 1049288
                        i32.add
                        i32.load
                        local.set 0
                      end
                      local.get 7
                      local.get 2
                      i32.store offset=8
                      local.get 0
                      local.get 2
                      i32.store offset=12
                      local.get 2
                      local.get 7
                      i32.store offset=12
                      local.get 2
                      local.get 0
                      i32.store offset=8
                      br 7 (;@2;)
                    end
                    local.get 0
                    local.get 6
                    i32.store
                    local.get 0
                    local.get 0
                    i32.load offset=4
                    local.get 9
                    i32.add
                    i32.store offset=4
                    local.get 6
                    i32.const 15
                    i32.add
                    i32.const -8
                    i32.and
                    i32.const -8
                    i32.add
                    local.tee 7
                    local.get 3
                    i32.const 3
                    i32.or
                    i32.store offset=4
                    local.get 8
                    i32.const 15
                    i32.add
                    i32.const -8
                    i32.and
                    i32.const -8
                    i32.add
                    local.tee 2
                    local.get 7
                    local.get 3
                    i32.add
                    local.tee 0
                    i32.sub
                    local.set 3
                    local.get 2
                    i32.const 0
                    i32.load offset=1049564
                    i32.eq
                    br_if 3 (;@5;)
                    local.get 2
                    i32.const 0
                    i32.load offset=1049560
                    i32.eq
                    br_if 4 (;@4;)
                    block  ;; label = @9
                      local.get 2
                      i32.load offset=4
                      local.tee 6
                      i32.const 3
                      i32.and
                      i32.const 1
                      i32.ne
                      br_if 0 (;@9;)
                      local.get 2
                      local.get 6
                      i32.const -8
                      i32.and
                      local.tee 6
                      call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$12unlink_chunk17h16ef10954c05020cE
                      local.get 6
                      local.get 3
                      i32.add
                      local.set 3
                      local.get 2
                      local.get 6
                      i32.add
                      local.tee 2
                      i32.load offset=4
                      local.set 6
                    end
                    local.get 2
                    local.get 6
                    i32.const -2
                    i32.and
                    i32.store offset=4
                    local.get 0
                    local.get 3
                    i32.const 1
                    i32.or
                    i32.store offset=4
                    local.get 0
                    local.get 3
                    i32.add
                    local.get 3
                    i32.store
                    block  ;; label = @9
                      local.get 3
                      i32.const 256
                      i32.lt_u
                      br_if 0 (;@9;)
                      local.get 0
                      local.get 3
                      call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$18insert_large_chunk17hc7cb11ca1345ed38E
                      br 6 (;@3;)
                    end
                    block  ;; label = @9
                      block  ;; label = @10
                        i32.const 0
                        i32.load offset=1049544
                        local.tee 2
                        i32.const 1
                        local.get 3
                        i32.const 3
                        i32.shr_u
                        i32.shl
                        local.tee 6
                        i32.and
                        br_if 0 (;@10;)
                        i32.const 0
                        local.get 2
                        local.get 6
                        i32.or
                        i32.store offset=1049544
                        local.get 3
                        i32.const 248
                        i32.and
                        i32.const 1049280
                        i32.add
                        local.tee 3
                        local.set 2
                        br 1 (;@9;)
                      end
                      local.get 3
                      i32.const 248
                      i32.and
                      local.tee 3
                      i32.const 1049280
                      i32.add
                      local.set 2
                      local.get 3
                      i32.const 1049288
                      i32.add
                      i32.load
                      local.set 3
                    end
                    local.get 2
                    local.get 0
                    i32.store offset=8
                    local.get 3
                    local.get 0
                    i32.store offset=12
                    local.get 0
                    local.get 2
                    i32.store offset=12
                    local.get 0
                    local.get 3
                    i32.store offset=8
                    br 5 (;@3;)
                  end
                  i32.const 0
                  local.get 0
                  local.get 3
                  i32.sub
                  local.tee 2
                  i32.store offset=1049556
                  i32.const 0
                  i32.const 0
                  i32.load offset=1049564
                  local.tee 0
                  local.get 3
                  i32.add
                  local.tee 7
                  i32.store offset=1049564
                  local.get 7
                  local.get 2
                  i32.const 1
                  i32.or
                  i32.store offset=4
                  local.get 0
                  local.get 3
                  i32.const 3
                  i32.or
                  i32.store offset=4
                  local.get 0
                  i32.const 8
                  i32.add
                  local.set 0
                  br 6 (;@1;)
                end
                i32.const 0
                i32.load offset=1049560
                local.set 2
                block  ;; label = @7
                  block  ;; label = @8
                    local.get 0
                    local.get 3
                    i32.sub
                    local.tee 7
                    i32.const 15
                    i32.gt_u
                    br_if 0 (;@8;)
                    i32.const 0
                    i32.const 0
                    i32.store offset=1049560
                    i32.const 0
                    i32.const 0
                    i32.store offset=1049552
                    local.get 2
                    local.get 0
                    i32.const 3
                    i32.or
                    i32.store offset=4
                    local.get 2
                    local.get 0
                    i32.add
                    local.tee 0
                    local.get 0
                    i32.load offset=4
                    i32.const 1
                    i32.or
                    i32.store offset=4
                    br 1 (;@7;)
                  end
                  i32.const 0
                  local.get 7
                  i32.store offset=1049552
                  i32.const 0
                  local.get 2
                  local.get 3
                  i32.add
                  local.tee 6
                  i32.store offset=1049560
                  local.get 6
                  local.get 7
                  i32.const 1
                  i32.or
                  i32.store offset=4
                  local.get 2
                  local.get 0
                  i32.add
                  local.get 7
                  i32.store
                  local.get 2
                  local.get 3
                  i32.const 3
                  i32.or
                  i32.store offset=4
                end
                local.get 2
                i32.const 8
                i32.add
                local.set 0
                br 5 (;@1;)
              end
              local.get 0
              local.get 8
              local.get 9
              i32.add
              i32.store offset=4
              i32.const 0
              i32.const 0
              i32.load offset=1049564
              local.tee 0
              i32.const 15
              i32.add
              i32.const -8
              i32.and
              local.tee 2
              i32.const -8
              i32.add
              local.tee 7
              i32.store offset=1049564
              i32.const 0
              local.get 0
              local.get 2
              i32.sub
              i32.const 0
              i32.load offset=1049556
              local.get 9
              i32.add
              local.tee 2
              i32.add
              i32.const 8
              i32.add
              local.tee 6
              i32.store offset=1049556
              local.get 7
              local.get 6
              i32.const 1
              i32.or
              i32.store offset=4
              local.get 0
              local.get 2
              i32.add
              i32.const 40
              i32.store offset=4
              i32.const 0
              i32.const 2097152
              i32.store offset=1049576
              br 3 (;@2;)
            end
            i32.const 0
            local.get 0
            i32.store offset=1049564
            i32.const 0
            i32.const 0
            i32.load offset=1049556
            local.get 3
            i32.add
            local.tee 3
            i32.store offset=1049556
            local.get 0
            local.get 3
            i32.const 1
            i32.or
            i32.store offset=4
            br 1 (;@3;)
          end
          i32.const 0
          local.get 0
          i32.store offset=1049560
          i32.const 0
          i32.const 0
          i32.load offset=1049552
          local.get 3
          i32.add
          local.tee 3
          i32.store offset=1049552
          local.get 0
          local.get 3
          i32.const 1
          i32.or
          i32.store offset=4
          local.get 0
          local.get 3
          i32.add
          local.get 3
          i32.store
        end
        local.get 7
        i32.const 8
        i32.add
        local.set 0
        br 1 (;@1;)
      end
      i32.const 0
      local.set 0
      i32.const 0
      i32.load offset=1049556
      local.tee 2
      local.get 3
      i32.le_u
      br_if 0 (;@1;)
      i32.const 0
      local.get 2
      local.get 3
      i32.sub
      local.tee 2
      i32.store offset=1049556
      i32.const 0
      i32.const 0
      i32.load offset=1049564
      local.tee 0
      local.get 3
      i32.add
      local.tee 7
      i32.store offset=1049564
      local.get 7
      local.get 2
      i32.const 1
      i32.or
      i32.store offset=4
      local.get 0
      local.get 3
      i32.const 3
      i32.or
      i32.store offset=4
      local.get 0
      i32.const 8
      i32.add
      local.set 0
    end
    local.get 1
    i32.const 16
    i32.add
    global.set $__stack_pointer
    local.get 0)
  (func $_RNvCs1Y7DaGC1cwg_7___rustc12___rust_abort (type 5)
    unreachable)
  (func $_RNvCs1Y7DaGC1cwg_7___rustc13___rdl_dealloc (type 6) (param i32 i32 i32)
    (local i32 i32)
    block  ;; label = @1
      block  ;; label = @2
        local.get 0
        i32.const -4
        i32.add
        i32.load
        local.tee 3
        i32.const -8
        i32.and
        local.tee 4
        i32.const 4
        i32.const 8
        local.get 3
        i32.const 3
        i32.and
        local.tee 3
        select
        local.get 1
        i32.add
        i32.lt_u
        br_if 0 (;@2;)
        block  ;; label = @3
          local.get 3
          i32.eqz
          br_if 0 (;@3;)
          local.get 4
          local.get 1
          i32.const 39
          i32.add
          i32.gt_u
          br_if 2 (;@1;)
        end
        local.get 0
        call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$4free17ha98daa4c6dd8ad86E
        return
      end
      i32.const 1048892
      i32.const 46
      i32.const 1048940
      call $_ZN4core9panicking5panic17h0149fc8f1656305aE
      unreachable
    end
    i32.const 1048956
    i32.const 46
    i32.const 1049004
    call $_ZN4core9panicking5panic17h0149fc8f1656305aE
    unreachable)
  (func $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$4free17ha98daa4c6dd8ad86E (type 8) (param i32)
    (local i32 i32 i32 i32 i32)
    local.get 0
    i32.const -8
    i32.add
    local.tee 1
    local.get 0
    i32.const -4
    i32.add
    i32.load
    local.tee 2
    i32.const -8
    i32.and
    local.tee 0
    i32.add
    local.set 3
    block  ;; label = @1
      block  ;; label = @2
        local.get 2
        i32.const 1
        i32.and
        br_if 0 (;@2;)
        local.get 2
        i32.const 2
        i32.and
        i32.eqz
        br_if 1 (;@1;)
        local.get 1
        i32.load
        local.tee 2
        local.get 0
        i32.add
        local.set 0
        block  ;; label = @3
          local.get 1
          local.get 2
          i32.sub
          local.tee 1
          i32.const 0
          i32.load offset=1049560
          i32.ne
          br_if 0 (;@3;)
          local.get 3
          i32.load offset=4
          i32.const 3
          i32.and
          i32.const 3
          i32.ne
          br_if 1 (;@2;)
          i32.const 0
          local.get 0
          i32.store offset=1049552
          local.get 3
          local.get 3
          i32.load offset=4
          i32.const -2
          i32.and
          i32.store offset=4
          local.get 1
          local.get 0
          i32.const 1
          i32.or
          i32.store offset=4
          local.get 3
          local.get 0
          i32.store
          return
        end
        local.get 1
        local.get 2
        call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$12unlink_chunk17h16ef10954c05020cE
      end
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  local.get 3
                  i32.load offset=4
                  local.tee 2
                  i32.const 2
                  i32.and
                  br_if 0 (;@7;)
                  local.get 3
                  i32.const 0
                  i32.load offset=1049564
                  i32.eq
                  br_if 2 (;@5;)
                  local.get 3
                  i32.const 0
                  i32.load offset=1049560
                  i32.eq
                  br_if 3 (;@4;)
                  local.get 3
                  local.get 2
                  i32.const -8
                  i32.and
                  local.tee 2
                  call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$12unlink_chunk17h16ef10954c05020cE
                  local.get 1
                  local.get 2
                  local.get 0
                  i32.add
                  local.tee 0
                  i32.const 1
                  i32.or
                  i32.store offset=4
                  local.get 1
                  local.get 0
                  i32.add
                  local.get 0
                  i32.store
                  local.get 1
                  i32.const 0
                  i32.load offset=1049560
                  i32.ne
                  br_if 1 (;@6;)
                  i32.const 0
                  local.get 0
                  i32.store offset=1049552
                  return
                end
                local.get 3
                local.get 2
                i32.const -2
                i32.and
                i32.store offset=4
                local.get 1
                local.get 0
                i32.const 1
                i32.or
                i32.store offset=4
                local.get 1
                local.get 0
                i32.add
                local.get 0
                i32.store
              end
              local.get 0
              i32.const 256
              i32.lt_u
              br_if 2 (;@3;)
              local.get 1
              local.get 0
              call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$18insert_large_chunk17hc7cb11ca1345ed38E
              i32.const 0
              local.set 1
              i32.const 0
              i32.const 0
              i32.load offset=1049584
              i32.const -1
              i32.add
              local.tee 0
              i32.store offset=1049584
              local.get 0
              br_if 4 (;@1;)
              block  ;; label = @6
                i32.const 0
                i32.load offset=1049272
                local.tee 0
                i32.eqz
                br_if 0 (;@6;)
                i32.const 0
                local.set 1
                loop  ;; label = @7
                  local.get 1
                  i32.const 1
                  i32.add
                  local.set 1
                  local.get 0
                  i32.load offset=8
                  local.tee 0
                  br_if 0 (;@7;)
                end
              end
              i32.const 0
              local.get 1
              i32.const 4095
              local.get 1
              i32.const 4095
              i32.gt_u
              select
              i32.store offset=1049584
              return
            end
            i32.const 0
            local.get 1
            i32.store offset=1049564
            i32.const 0
            i32.const 0
            i32.load offset=1049556
            local.get 0
            i32.add
            local.tee 0
            i32.store offset=1049556
            local.get 1
            local.get 0
            i32.const 1
            i32.or
            i32.store offset=4
            block  ;; label = @5
              local.get 1
              i32.const 0
              i32.load offset=1049560
              i32.ne
              br_if 0 (;@5;)
              i32.const 0
              i32.const 0
              i32.store offset=1049552
              i32.const 0
              i32.const 0
              i32.store offset=1049560
            end
            local.get 0
            i32.const 0
            i32.load offset=1049576
            local.tee 4
            i32.le_u
            br_if 3 (;@1;)
            i32.const 0
            i32.load offset=1049564
            local.tee 0
            i32.eqz
            br_if 3 (;@1;)
            i32.const 0
            local.set 2
            i32.const 0
            i32.load offset=1049556
            local.tee 5
            i32.const 41
            i32.lt_u
            br_if 2 (;@2;)
            i32.const 1049264
            local.set 1
            loop  ;; label = @5
              block  ;; label = @6
                local.get 1
                i32.load
                local.tee 3
                local.get 0
                i32.gt_u
                br_if 0 (;@6;)
                local.get 0
                local.get 3
                local.get 1
                i32.load offset=4
                i32.add
                i32.lt_u
                br_if 4 (;@2;)
              end
              local.get 1
              i32.load offset=8
              local.set 1
              br 0 (;@5;)
            end
          end
          i32.const 0
          local.get 1
          i32.store offset=1049560
          i32.const 0
          i32.const 0
          i32.load offset=1049552
          local.get 0
          i32.add
          local.tee 0
          i32.store offset=1049552
          local.get 1
          local.get 0
          i32.const 1
          i32.or
          i32.store offset=4
          local.get 1
          local.get 0
          i32.add
          local.get 0
          i32.store
          return
        end
        block  ;; label = @3
          block  ;; label = @4
            i32.const 0
            i32.load offset=1049544
            local.tee 3
            i32.const 1
            local.get 0
            i32.const 3
            i32.shr_u
            i32.shl
            local.tee 2
            i32.and
            br_if 0 (;@4;)
            i32.const 0
            local.get 3
            local.get 2
            i32.or
            i32.store offset=1049544
            local.get 0
            i32.const 248
            i32.and
            i32.const 1049280
            i32.add
            local.tee 0
            local.set 3
            br 1 (;@3;)
          end
          local.get 0
          i32.const 248
          i32.and
          local.tee 0
          i32.const 1049280
          i32.add
          local.set 3
          local.get 0
          i32.const 1049288
          i32.add
          i32.load
          local.set 0
        end
        local.get 3
        local.get 1
        i32.store offset=8
        local.get 0
        local.get 1
        i32.store offset=12
        local.get 1
        local.get 3
        i32.store offset=12
        local.get 1
        local.get 0
        i32.store offset=8
        return
      end
      block  ;; label = @2
        i32.const 0
        i32.load offset=1049272
        local.tee 1
        i32.eqz
        br_if 0 (;@2;)
        i32.const 0
        local.set 2
        loop  ;; label = @3
          local.get 2
          i32.const 1
          i32.add
          local.set 2
          local.get 1
          i32.load offset=8
          local.tee 1
          br_if 0 (;@3;)
        end
      end
      i32.const 0
      local.get 2
      i32.const 4095
      local.get 2
      i32.const 4095
      i32.gt_u
      select
      i32.store offset=1049584
      local.get 5
      local.get 4
      i32.le_u
      br_if 0 (;@1;)
      i32.const 0
      i32.const -1
      i32.store offset=1049576
    end)
  (func $_RNvCs1Y7DaGC1cwg_7___rustc13___rdl_realloc (type 7) (param i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32)
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    local.get 0
                    i32.const -4
                    i32.add
                    local.tee 4
                    i32.load
                    local.tee 5
                    i32.const -8
                    i32.and
                    local.tee 6
                    i32.const 4
                    i32.const 8
                    local.get 5
                    i32.const 3
                    i32.and
                    local.tee 7
                    select
                    local.get 1
                    i32.add
                    i32.lt_u
                    br_if 0 (;@8;)
                    local.get 1
                    i32.const 39
                    i32.add
                    local.set 8
                    block  ;; label = @9
                      local.get 7
                      i32.eqz
                      br_if 0 (;@9;)
                      local.get 6
                      local.get 8
                      i32.gt_u
                      br_if 2 (;@7;)
                    end
                    block  ;; label = @9
                      block  ;; label = @10
                        local.get 2
                        i32.const 9
                        i32.lt_u
                        br_if 0 (;@10;)
                        local.get 2
                        local.get 3
                        call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$8memalign17h4d574c3a3414c418E
                        local.tee 2
                        br_if 1 (;@9;)
                        i32.const 0
                        return
                      end
                      i32.const 0
                      local.set 2
                      local.get 3
                      i32.const -65588
                      i32.gt_u
                      br_if 8 (;@1;)
                      i32.const 16
                      local.get 3
                      i32.const 11
                      i32.add
                      i32.const -8
                      i32.and
                      local.get 3
                      i32.const 11
                      i32.lt_u
                      select
                      local.set 1
                      local.get 0
                      i32.const -8
                      i32.add
                      local.set 8
                      block  ;; label = @10
                        local.get 7
                        br_if 0 (;@10;)
                        local.get 1
                        i32.const 256
                        i32.lt_u
                        br_if 7 (;@3;)
                        local.get 8
                        i32.eqz
                        br_if 7 (;@3;)
                        local.get 6
                        local.get 1
                        i32.le_u
                        br_if 7 (;@3;)
                        local.get 6
                        local.get 1
                        i32.sub
                        i32.const 131072
                        i32.gt_u
                        br_if 7 (;@3;)
                        local.get 0
                        return
                      end
                      local.get 8
                      local.get 6
                      i32.add
                      local.set 7
                      block  ;; label = @10
                        block  ;; label = @11
                          local.get 6
                          local.get 1
                          i32.ge_u
                          br_if 0 (;@11;)
                          local.get 7
                          i32.const 0
                          i32.load offset=1049564
                          i32.eq
                          br_if 1 (;@10;)
                          block  ;; label = @12
                            local.get 7
                            i32.const 0
                            i32.load offset=1049560
                            i32.eq
                            br_if 0 (;@12;)
                            local.get 7
                            i32.load offset=4
                            local.tee 5
                            i32.const 2
                            i32.and
                            br_if 9 (;@3;)
                            local.get 5
                            i32.const -8
                            i32.and
                            local.tee 9
                            local.get 6
                            i32.add
                            local.tee 5
                            local.get 1
                            i32.lt_u
                            br_if 9 (;@3;)
                            local.get 7
                            local.get 9
                            call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$12unlink_chunk17h16ef10954c05020cE
                            block  ;; label = @13
                              local.get 5
                              local.get 1
                              i32.sub
                              local.tee 7
                              i32.const 16
                              i32.lt_u
                              br_if 0 (;@13;)
                              local.get 4
                              local.get 1
                              local.get 4
                              i32.load
                              i32.const 1
                              i32.and
                              i32.or
                              i32.const 2
                              i32.or
                              i32.store
                              local.get 8
                              local.get 1
                              i32.add
                              local.tee 1
                              local.get 7
                              i32.const 3
                              i32.or
                              i32.store offset=4
                              local.get 8
                              local.get 5
                              i32.add
                              local.tee 5
                              local.get 5
                              i32.load offset=4
                              i32.const 1
                              i32.or
                              i32.store offset=4
                              local.get 1
                              local.get 7
                              call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$13dispose_chunk17h0e84f108fd9f7b7cE
                              br 9 (;@4;)
                            end
                            local.get 4
                            local.get 5
                            local.get 4
                            i32.load
                            i32.const 1
                            i32.and
                            i32.or
                            i32.const 2
                            i32.or
                            i32.store
                            local.get 8
                            local.get 5
                            i32.add
                            local.tee 1
                            local.get 1
                            i32.load offset=4
                            i32.const 1
                            i32.or
                            i32.store offset=4
                            br 8 (;@4;)
                          end
                          i32.const 0
                          i32.load offset=1049552
                          local.get 6
                          i32.add
                          local.tee 7
                          local.get 1
                          i32.lt_u
                          br_if 8 (;@3;)
                          block  ;; label = @12
                            block  ;; label = @13
                              local.get 7
                              local.get 1
                              i32.sub
                              local.tee 6
                              i32.const 15
                              i32.gt_u
                              br_if 0 (;@13;)
                              local.get 4
                              local.get 5
                              i32.const 1
                              i32.and
                              local.get 7
                              i32.or
                              i32.const 2
                              i32.or
                              i32.store
                              local.get 8
                              local.get 7
                              i32.add
                              local.tee 1
                              local.get 1
                              i32.load offset=4
                              i32.const 1
                              i32.or
                              i32.store offset=4
                              i32.const 0
                              local.set 6
                              i32.const 0
                              local.set 1
                              br 1 (;@12;)
                            end
                            local.get 4
                            local.get 1
                            local.get 5
                            i32.const 1
                            i32.and
                            i32.or
                            i32.const 2
                            i32.or
                            i32.store
                            local.get 8
                            local.get 1
                            i32.add
                            local.tee 1
                            local.get 6
                            i32.const 1
                            i32.or
                            i32.store offset=4
                            local.get 8
                            local.get 7
                            i32.add
                            local.tee 7
                            local.get 6
                            i32.store
                            local.get 7
                            local.get 7
                            i32.load offset=4
                            i32.const -2
                            i32.and
                            i32.store offset=4
                          end
                          i32.const 0
                          local.get 1
                          i32.store offset=1049560
                          i32.const 0
                          local.get 6
                          i32.store offset=1049552
                          br 7 (;@4;)
                        end
                        local.get 6
                        local.get 1
                        i32.sub
                        local.tee 6
                        i32.const 15
                        i32.le_u
                        br_if 6 (;@4;)
                        local.get 4
                        local.get 1
                        local.get 5
                        i32.const 1
                        i32.and
                        i32.or
                        i32.const 2
                        i32.or
                        i32.store
                        local.get 8
                        local.get 1
                        i32.add
                        local.tee 1
                        local.get 6
                        i32.const 3
                        i32.or
                        i32.store offset=4
                        local.get 7
                        local.get 7
                        i32.load offset=4
                        i32.const 1
                        i32.or
                        i32.store offset=4
                        local.get 1
                        local.get 6
                        call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$13dispose_chunk17h0e84f108fd9f7b7cE
                        br 6 (;@4;)
                      end
                      i32.const 0
                      i32.load offset=1049556
                      local.get 6
                      i32.add
                      local.tee 7
                      local.get 1
                      i32.gt_u
                      br_if 4 (;@5;)
                      br 6 (;@3;)
                    end
                    block  ;; label = @9
                      local.get 3
                      local.get 1
                      local.get 3
                      local.get 1
                      i32.lt_u
                      select
                      local.tee 3
                      i32.eqz
                      br_if 0 (;@9;)
                      local.get 2
                      local.get 0
                      local.get 3
                      memory.copy
                    end
                    local.get 4
                    i32.load
                    local.tee 3
                    i32.const -8
                    i32.and
                    local.tee 7
                    i32.const 4
                    i32.const 8
                    local.get 3
                    i32.const 3
                    i32.and
                    local.tee 3
                    select
                    local.get 1
                    i32.add
                    i32.lt_u
                    br_if 2 (;@6;)
                    local.get 3
                    i32.eqz
                    br_if 6 (;@2;)
                    local.get 7
                    local.get 8
                    i32.le_u
                    br_if 6 (;@2;)
                    i32.const 1048956
                    i32.const 46
                    i32.const 1049004
                    call $_ZN4core9panicking5panic17h0149fc8f1656305aE
                    unreachable
                  end
                  i32.const 1048892
                  i32.const 46
                  i32.const 1048940
                  call $_ZN4core9panicking5panic17h0149fc8f1656305aE
                  unreachable
                end
                i32.const 1048956
                i32.const 46
                i32.const 1049004
                call $_ZN4core9panicking5panic17h0149fc8f1656305aE
                unreachable
              end
              i32.const 1048892
              i32.const 46
              i32.const 1048940
              call $_ZN4core9panicking5panic17h0149fc8f1656305aE
              unreachable
            end
            local.get 4
            local.get 1
            local.get 5
            i32.const 1
            i32.and
            i32.or
            i32.const 2
            i32.or
            i32.store
            local.get 8
            local.get 1
            i32.add
            local.tee 5
            local.get 7
            local.get 1
            i32.sub
            local.tee 1
            i32.const 1
            i32.or
            i32.store offset=4
            i32.const 0
            local.get 1
            i32.store offset=1049556
            i32.const 0
            local.get 5
            i32.store offset=1049564
          end
          local.get 8
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          return
        end
        local.get 3
        call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$6malloc17he97e96981fab807eE
        local.tee 1
        i32.eqz
        br_if 1 (;@1;)
        block  ;; label = @3
          local.get 3
          i32.const -4
          i32.const -8
          local.get 4
          i32.load
          local.tee 2
          i32.const 3
          i32.and
          select
          local.get 2
          i32.const -8
          i32.and
          i32.add
          local.tee 2
          local.get 3
          local.get 2
          i32.lt_u
          select
          local.tee 3
          i32.eqz
          br_if 0 (;@3;)
          local.get 1
          local.get 0
          local.get 3
          memory.copy
        end
        local.get 1
        local.set 2
      end
      local.get 0
      call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$4free17ha98daa4c6dd8ad86E
    end
    local.get 2)
  (func $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$12unlink_chunk17h16ef10954c05020cE (type 0) (param i32 i32)
    (local i32 i32 i32 i32)
    local.get 0
    i32.load offset=12
    local.set 2
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            local.get 1
            i32.const 256
            i32.lt_u
            br_if 0 (;@4;)
            local.get 0
            i32.load offset=24
            local.set 3
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  local.get 2
                  local.get 0
                  i32.ne
                  br_if 0 (;@7;)
                  local.get 0
                  i32.const 20
                  i32.const 16
                  local.get 0
                  i32.load offset=20
                  local.tee 2
                  select
                  i32.add
                  i32.load
                  local.tee 1
                  br_if 1 (;@6;)
                  i32.const 0
                  local.set 2
                  br 2 (;@5;)
                end
                local.get 0
                i32.load offset=8
                local.tee 1
                local.get 2
                i32.store offset=12
                local.get 2
                local.get 1
                i32.store offset=8
                br 1 (;@5;)
              end
              local.get 0
              i32.const 20
              i32.add
              local.get 0
              i32.const 16
              i32.add
              local.get 2
              select
              local.set 4
              loop  ;; label = @6
                local.get 4
                local.set 5
                local.get 1
                local.tee 2
                i32.const 20
                i32.add
                local.get 2
                i32.const 16
                i32.add
                local.get 2
                i32.load offset=20
                local.tee 1
                select
                local.set 4
                local.get 2
                i32.const 20
                i32.const 16
                local.get 1
                select
                i32.add
                i32.load
                local.tee 1
                br_if 0 (;@6;)
              end
              local.get 5
              i32.const 0
              i32.store
            end
            local.get 3
            i32.eqz
            br_if 2 (;@2;)
            block  ;; label = @5
              block  ;; label = @6
                local.get 0
                local.get 0
                i32.load offset=28
                i32.const 2
                i32.shl
                i32.const 1049136
                i32.add
                local.tee 1
                i32.load
                i32.eq
                br_if 0 (;@6;)
                local.get 3
                i32.load offset=16
                local.get 0
                i32.eq
                br_if 1 (;@5;)
                local.get 3
                local.get 2
                i32.store offset=20
                local.get 2
                br_if 3 (;@3;)
                br 4 (;@2;)
              end
              local.get 1
              local.get 2
              i32.store
              local.get 2
              i32.eqz
              br_if 4 (;@1;)
              br 2 (;@3;)
            end
            local.get 3
            local.get 2
            i32.store offset=16
            local.get 2
            br_if 1 (;@3;)
            br 2 (;@2;)
          end
          block  ;; label = @4
            local.get 2
            local.get 0
            i32.load offset=8
            local.tee 4
            i32.eq
            br_if 0 (;@4;)
            local.get 4
            local.get 2
            i32.store offset=12
            local.get 2
            local.get 4
            i32.store offset=8
            return
          end
          i32.const 0
          i32.const 0
          i32.load offset=1049544
          i32.const -2
          local.get 1
          i32.const 3
          i32.shr_u
          i32.rotl
          i32.and
          i32.store offset=1049544
          return
        end
        local.get 2
        local.get 3
        i32.store offset=24
        block  ;; label = @3
          local.get 0
          i32.load offset=16
          local.tee 1
          i32.eqz
          br_if 0 (;@3;)
          local.get 2
          local.get 1
          i32.store offset=16
          local.get 1
          local.get 2
          i32.store offset=24
        end
        local.get 0
        i32.load offset=20
        local.tee 1
        i32.eqz
        br_if 0 (;@2;)
        local.get 2
        local.get 1
        i32.store offset=20
        local.get 1
        local.get 2
        i32.store offset=24
        return
      end
      return
    end
    i32.const 0
    i32.const 0
    i32.load offset=1049548
    i32.const -2
    local.get 0
    i32.load offset=28
    i32.rotl
    i32.and
    i32.store offset=1049548)
  (func $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$13dispose_chunk17h0e84f108fd9f7b7cE (type 0) (param i32 i32)
    (local i32 i32)
    local.get 0
    local.get 1
    i32.add
    local.set 2
    block  ;; label = @1
      block  ;; label = @2
        local.get 0
        i32.load offset=4
        local.tee 3
        i32.const 1
        i32.and
        br_if 0 (;@2;)
        local.get 3
        i32.const 2
        i32.and
        i32.eqz
        br_if 1 (;@1;)
        local.get 0
        i32.load
        local.tee 3
        local.get 1
        i32.add
        local.set 1
        block  ;; label = @3
          local.get 0
          local.get 3
          i32.sub
          local.tee 0
          i32.const 0
          i32.load offset=1049560
          i32.ne
          br_if 0 (;@3;)
          local.get 2
          i32.load offset=4
          i32.const 3
          i32.and
          i32.const 3
          i32.ne
          br_if 1 (;@2;)
          i32.const 0
          local.get 1
          i32.store offset=1049552
          local.get 2
          local.get 2
          i32.load offset=4
          i32.const -2
          i32.and
          i32.store offset=4
          local.get 0
          local.get 1
          i32.const 1
          i32.or
          i32.store offset=4
          local.get 2
          local.get 1
          i32.store
          br 2 (;@1;)
        end
        local.get 0
        local.get 3
        call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$12unlink_chunk17h16ef10954c05020cE
      end
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 2
              i32.load offset=4
              local.tee 3
              i32.const 2
              i32.and
              br_if 0 (;@5;)
              local.get 2
              i32.const 0
              i32.load offset=1049564
              i32.eq
              br_if 2 (;@3;)
              local.get 2
              i32.const 0
              i32.load offset=1049560
              i32.eq
              br_if 3 (;@2;)
              local.get 2
              local.get 3
              i32.const -8
              i32.and
              local.tee 3
              call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$12unlink_chunk17h16ef10954c05020cE
              local.get 0
              local.get 3
              local.get 1
              i32.add
              local.tee 1
              i32.const 1
              i32.or
              i32.store offset=4
              local.get 0
              local.get 1
              i32.add
              local.get 1
              i32.store
              local.get 0
              i32.const 0
              i32.load offset=1049560
              i32.ne
              br_if 1 (;@4;)
              i32.const 0
              local.get 1
              i32.store offset=1049552
              return
            end
            local.get 2
            local.get 3
            i32.const -2
            i32.and
            i32.store offset=4
            local.get 0
            local.get 1
            i32.const 1
            i32.or
            i32.store offset=4
            local.get 0
            local.get 1
            i32.add
            local.get 1
            i32.store
          end
          block  ;; label = @4
            local.get 1
            i32.const 256
            i32.lt_u
            br_if 0 (;@4;)
            local.get 0
            local.get 1
            call $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$18insert_large_chunk17hc7cb11ca1345ed38E
            return
          end
          block  ;; label = @4
            block  ;; label = @5
              i32.const 0
              i32.load offset=1049544
              local.tee 2
              i32.const 1
              local.get 1
              i32.const 3
              i32.shr_u
              i32.shl
              local.tee 3
              i32.and
              br_if 0 (;@5;)
              i32.const 0
              local.get 2
              local.get 3
              i32.or
              i32.store offset=1049544
              local.get 1
              i32.const 248
              i32.and
              i32.const 1049280
              i32.add
              local.tee 1
              local.set 2
              br 1 (;@4;)
            end
            local.get 1
            i32.const 248
            i32.and
            local.tee 1
            i32.const 1049280
            i32.add
            local.set 2
            local.get 1
            i32.const 1049288
            i32.add
            i32.load
            local.set 1
          end
          local.get 2
          local.get 0
          i32.store offset=8
          local.get 1
          local.get 0
          i32.store offset=12
          local.get 0
          local.get 2
          i32.store offset=12
          local.get 0
          local.get 1
          i32.store offset=8
          return
        end
        i32.const 0
        local.get 0
        i32.store offset=1049564
        i32.const 0
        i32.const 0
        i32.load offset=1049556
        local.get 1
        i32.add
        local.tee 1
        i32.store offset=1049556
        local.get 0
        local.get 1
        i32.const 1
        i32.or
        i32.store offset=4
        local.get 0
        i32.const 0
        i32.load offset=1049560
        i32.ne
        br_if 1 (;@1;)
        i32.const 0
        i32.const 0
        i32.store offset=1049552
        i32.const 0
        i32.const 0
        i32.store offset=1049560
        return
      end
      i32.const 0
      local.get 0
      i32.store offset=1049560
      i32.const 0
      i32.const 0
      i32.load offset=1049552
      local.get 1
      i32.add
      local.tee 1
      i32.store offset=1049552
      local.get 0
      local.get 1
      i32.const 1
      i32.or
      i32.store offset=4
      local.get 0
      local.get 1
      i32.add
      local.get 1
      i32.store
      return
    end)
  (func $_RNvCs1Y7DaGC1cwg_7___rustc17rust_begin_unwind (type 8) (param i32)
    (local i32 i64)
    global.get $__stack_pointer
    i32.const 16
    i32.sub
    local.tee 1
    global.set $__stack_pointer
    local.get 0
    i64.load align=4
    local.set 2
    local.get 1
    local.get 0
    i32.store offset=12
    local.get 1
    local.get 2
    i64.store offset=4 align=4
    local.get 1
    i32.const 4
    i32.add
    call $_ZN3std3sys9backtrace26__rust_end_short_backtrace17h46ab1174c51ef229E
    unreachable)
  (func $_ZN3std3sys9backtrace26__rust_end_short_backtrace17h46ab1174c51ef229E (type 8) (param i32)
    local.get 0
    call $_ZN3std9panicking13panic_handler28_$u7b$$u7b$closure$u7d$$u7d$17hf36efc37fdd11196E
    unreachable)
  (func $_RNvCs1Y7DaGC1cwg_7___rustc26___rust_alloc_error_handler (type 0) (param i32 i32)
    local.get 1
    local.get 0
    call $_ZN3std5alloc8rust_oom17hbbbc0258d349aa94E
    unreachable)
  (func $_ZN3std5alloc8rust_oom17hbbbc0258d349aa94E (type 0) (param i32 i32)
    (local i32)
    global.get $__stack_pointer
    i32.const 16
    i32.sub
    local.tee 2
    global.set $__stack_pointer
    local.get 2
    local.get 1
    i32.store offset=12
    local.get 2
    local.get 0
    i32.store offset=8
    local.get 2
    i32.const 8
    i32.add
    call $_ZN3std3sys9backtrace26__rust_end_short_backtrace17h81a6ec13af23b259E
    unreachable)
  (func $_ZN36_$LT$T$u20$as$u20$core..any..Any$GT$7type_id17h054965a0d695c3aeE (type 0) (param i32 i32)
    local.get 0
    i32.const 8
    i32.add
    i32.const 0
    i64.load offset=1048772 align=4
    i64.store align=4
    local.get 0
    i32.const 0
    i64.load offset=1048764 align=4
    i64.store align=4)
  (func $_ZN36_$LT$T$u20$as$u20$core..any..Any$GT$7type_id17h81f53a1fd46151f1E (type 0) (param i32 i32)
    local.get 0
    i32.const 8
    i32.add
    i32.const 0
    i64.load offset=1048788 align=4
    i64.store align=4
    local.get 0
    i32.const 0
    i64.load offset=1048780 align=4
    i64.store align=4)
  (func $_ZN5alloc7raw_vec20RawVecInner$LT$A$GT$7reserve21do_reserve_and_handle17hcef08ec0cc696d81E (type 9) (param i32 i32 i32 i32 i32)
    (local i32)
    global.get $__stack_pointer
    i32.const 16
    i32.sub
    local.tee 5
    global.set $__stack_pointer
    block  ;; label = @1
      local.get 2
      local.get 1
      i32.add
      local.tee 1
      local.get 2
      i32.ge_u
      br_if 0 (;@1;)
      i32.const 0
      i32.const 0
      call $_ZN5alloc7raw_vec12handle_error17h9ace31a903e6893eE
      unreachable
    end
    local.get 5
    i32.const 4
    i32.add
    local.get 0
    i32.load
    local.tee 2
    local.get 0
    i32.load offset=4
    local.get 1
    local.get 2
    i32.const 1
    i32.shl
    local.tee 2
    local.get 1
    local.get 2
    i32.gt_u
    select
    local.tee 2
    i32.const 8
    i32.const 4
    local.get 4
    i32.const 1
    i32.eq
    select
    local.tee 1
    local.get 2
    local.get 1
    i32.gt_u
    select
    local.tee 2
    local.get 3
    local.get 4
    call $_ZN5alloc7raw_vec20RawVecInner$LT$A$GT$11finish_grow17h4217bbf3026c2f59E
    block  ;; label = @1
      local.get 5
      i32.load offset=4
      i32.const 1
      i32.ne
      br_if 0 (;@1;)
      local.get 5
      i32.load offset=8
      local.get 5
      i32.load offset=12
      call $_ZN5alloc7raw_vec12handle_error17h9ace31a903e6893eE
      unreachable
    end
    local.get 5
    i32.load offset=8
    local.set 4
    local.get 0
    local.get 2
    i32.store
    local.get 0
    local.get 4
    i32.store offset=4
    local.get 5
    i32.const 16
    i32.add
    global.set $__stack_pointer)
  (func $_ZN3std9panicking13panic_handler28_$u7b$$u7b$closure$u7d$$u7d$17hf36efc37fdd11196E (type 8) (param i32)
    (local i32 i32 i32)
    global.get $__stack_pointer
    i32.const 16
    i32.sub
    local.tee 1
    global.set $__stack_pointer
    block  ;; label = @1
      local.get 0
      i32.load
      local.tee 2
      i32.load offset=4
      local.tee 3
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 2
      i32.load
      local.set 2
      local.get 1
      local.get 3
      i32.const 1
      i32.shr_u
      i32.store offset=4
      local.get 1
      local.get 2
      i32.store
      local.get 1
      i32.const 1048820
      local.get 0
      i32.load offset=4
      local.get 0
      i32.load offset=8
      local.tee 0
      i32.load8_u offset=8
      local.get 0
      i32.load8_u offset=9
      call $_ZN3std9panicking15panic_with_hook17h77afe0ddfda2cb89E
      unreachable
    end
    local.get 1
    i32.const -2147483648
    i32.store
    local.get 1
    local.get 0
    i32.store offset=12
    local.get 1
    i32.const 1048848
    local.get 0
    i32.load offset=4
    local.get 0
    i32.load offset=8
    local.tee 0
    i32.load8_u offset=8
    local.get 0
    i32.load8_u offset=9
    call $_ZN3std9panicking15panic_with_hook17h77afe0ddfda2cb89E
    unreachable)
  (func $_ZN3std3sys9backtrace26__rust_end_short_backtrace17h81a6ec13af23b259E (type 8) (param i32)
    local.get 0
    call $_ZN3std5alloc8rust_oom28_$u7b$$u7b$closure$u7d$$u7d$17hd75745f6cab4b1f8E
    unreachable)
  (func $_ZN3std5alloc8rust_oom28_$u7b$$u7b$closure$u7d$$u7d$17hd75745f6cab4b1f8E (type 8) (param i32)
    local.get 0
    i32.load
    local.get 0
    i32.load offset=4
    i32.const 0
    i32.load offset=1049592
    local.tee 0
    i32.const 1
    local.get 0
    select
    call_indirect (type 0)
    unreachable)
  (func $_ZN5alloc7raw_vec20RawVecInner$LT$A$GT$11finish_grow17h4217bbf3026c2f59E (type 10) (param i32 i32 i32 i32 i32 i32)
    (local i32 i32 i64)
    i32.const 1
    local.set 6
    i32.const 4
    local.set 7
    block  ;; label = @1
      block  ;; label = @2
        local.get 4
        local.get 5
        i32.add
        i32.const -1
        i32.add
        i32.const 0
        local.get 4
        i32.sub
        i32.and
        i64.extend_i32_u
        local.get 3
        i64.extend_i32_u
        i64.mul
        local.tee 8
        i64.const 32
        i64.shr_u
        i32.wrap_i64
        i32.eqz
        br_if 0 (;@2;)
        i32.const 0
        local.set 3
        br 1 (;@1;)
      end
      block  ;; label = @2
        local.get 8
        i32.wrap_i64
        local.tee 3
        i32.const -2147483648
        local.get 4
        i32.sub
        i32.le_u
        br_if 0 (;@2;)
        i32.const 0
        local.set 3
        br 1 (;@1;)
      end
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 1
              i32.eqz
              br_if 0 (;@5;)
              local.get 2
              local.get 5
              local.get 1
              i32.mul
              local.get 4
              local.get 3
              call $_RNvCs1Y7DaGC1cwg_7___rustc14___rust_realloc
              local.set 7
              br 1 (;@4;)
            end
            block  ;; label = @5
              local.get 3
              br_if 0 (;@5;)
              local.get 4
              local.set 7
              br 2 (;@3;)
            end
            call $_RNvCs1Y7DaGC1cwg_7___rustc35___rust_no_alloc_shim_is_unstable_v2
            local.get 3
            local.get 4
            call $_RNvCs1Y7DaGC1cwg_7___rustc12___rust_alloc
            local.set 7
          end
          local.get 7
          br_if 0 (;@3;)
          local.get 0
          local.get 4
          i32.store offset=4
          br 1 (;@2;)
        end
        local.get 0
        local.get 7
        i32.store offset=4
        i32.const 0
        local.set 6
      end
      i32.const 8
      local.set 7
    end
    local.get 0
    local.get 7
    i32.add
    local.get 3
    i32.store
    local.get 0
    local.get 6
    i32.store)
  (func $_ZN3std5alloc24default_alloc_error_hook17h4ae318a4060b4b2eE (type 0) (param i32 i32)
    i32.const 0
    i32.const 1
    i32.store8 offset=1049588)
  (func $_ZN3std9panicking15panic_with_hook17h77afe0ddfda2cb89E (type 9) (param i32 i32 i32 i32 i32)
    (local i32 i32)
    global.get $__stack_pointer
    i32.const 32
    i32.sub
    local.tee 5
    global.set $__stack_pointer
    block  ;; label = @1
      block  ;; label = @2
        i32.const 1
        call $_ZN3std9panicking11panic_count8increase17h540a4f946b6e7ad5E
        i32.const 255
        i32.and
        local.tee 6
        i32.const 2
        i32.eq
        br_if 0 (;@2;)
        local.get 6
        i32.const 1
        i32.and
        i32.eqz
        br_if 1 (;@1;)
        local.get 5
        i32.const 8
        i32.add
        local.get 0
        local.get 1
        i32.load offset=24
        call_indirect (type 0)
        br 1 (;@1;)
      end
      i32.const 0
      i32.load offset=1049608
      local.tee 6
      i32.const -1
      i32.le_s
      br_if 0 (;@1;)
      i32.const 0
      local.get 6
      i32.const 1
      i32.add
      i32.store offset=1049608
      block  ;; label = @2
        block  ;; label = @3
          i32.const 0
          i32.load offset=1049612
          i32.eqz
          br_if 0 (;@3;)
          local.get 5
          local.get 0
          local.get 1
          i32.load offset=20
          call_indirect (type 0)
          local.get 5
          local.get 4
          i32.store8 offset=29
          local.get 5
          local.get 3
          i32.store8 offset=28
          local.get 5
          local.get 2
          i32.store offset=24
          local.get 5
          local.get 5
          i64.load
          i64.store offset=16 align=4
          i32.const 0
          i32.load offset=1049612
          local.get 5
          i32.const 16
          i32.add
          i32.const 0
          i32.load offset=1049616
          i32.load offset=20
          call_indirect (type 0)
          br 1 (;@2;)
        end
        i32.const -2147483648
        local.get 5
        call $_ZN4core3ptr74drop_in_place$LT$core..option..Option$LT$alloc..vec..Vec$LT$u8$GT$$GT$$GT$17hd8232a1684e31e26E
      end
      i32.const 0
      i32.const 0
      i32.load offset=1049608
      i32.const -1
      i32.add
      i32.store offset=1049608
      i32.const 0
      i32.const 0
      i32.store8 offset=1049600
      local.get 3
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      local.get 1
      call $_RNvCs1Y7DaGC1cwg_7___rustc10rust_panic
      unreachable
    end
    unreachable)
  (func $_ZN3std9panicking11panic_count8increase17h540a4f946b6e7ad5E (type 3) (param i32) (result i32)
    (local i32 i32)
    i32.const 0
    local.set 1
    i32.const 0
    i32.const 0
    i32.load offset=1049604
    local.tee 2
    i32.const 1
    i32.add
    i32.store offset=1049604
    block  ;; label = @1
      local.get 2
      i32.const 0
      i32.lt_s
      br_if 0 (;@1;)
      i32.const 1
      local.set 1
      i32.const 0
      i32.load8_u offset=1049600
      br_if 0 (;@1;)
      i32.const 0
      local.get 0
      i32.store8 offset=1049600
      i32.const 0
      i32.const 0
      i32.load offset=1049596
      i32.const 1
      i32.add
      i32.store offset=1049596
      i32.const 2
      local.set 1
    end
    local.get 1)
  (func $_ZN4core3ptr74drop_in_place$LT$core..option..Option$LT$alloc..vec..Vec$LT$u8$GT$$GT$$GT$17hd8232a1684e31e26E (type 0) (param i32 i32)
    block  ;; label = @1
      local.get 0
      i32.const -2147483648
      i32.or
      i32.const -2147483648
      i32.eq
      br_if 0 (;@1;)
      local.get 1
      local.get 0
      i32.const 1
      call $_RNvCs1Y7DaGC1cwg_7___rustc14___rust_dealloc
    end)
  (func $_ZN4core3fmt5Write9write_fmt17h628b111ce4addafbE (type 1) (param i32 i32 i32) (result i32)
    local.get 0
    i32.const 1048796
    local.get 1
    local.get 2
    call $_ZN4core3fmt5write17h31474238f266a14aE)
  (func $_ZN4core3ptr42drop_in_place$LT$alloc..string..String$GT$17h5f918006ef5d0ce0E (type 8) (param i32)
    (local i32)
    block  ;; label = @1
      local.get 0
      i32.load
      local.tee 1
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      i32.load offset=4
      local.get 1
      i32.const 1
      call $_RNvCs1Y7DaGC1cwg_7___rustc14___rust_dealloc
    end)
  (func $_ZN4core3ptr71drop_in_place$LT$std..panicking..panic_handler..FormatStringPayload$GT$17hf7b8d0e0fb4c83c3E (type 8) (param i32)
    (local i32)
    block  ;; label = @1
      local.get 0
      i32.load
      local.tee 1
      i32.const -2147483648
      i32.or
      i32.const -2147483648
      i32.eq
      br_if 0 (;@1;)
      local.get 0
      i32.load offset=4
      local.get 1
      i32.const 1
      call $_RNvCs1Y7DaGC1cwg_7___rustc14___rust_dealloc
    end)
  (func $_ZN4core5panic12PanicPayload6as_str17h17c1d5120cb39a3fE (type 0) (param i32 i32)
    local.get 0
    i32.const 0
    i32.store)
  (func $_ZN58_$LT$alloc..string..String$u20$as$u20$core..fmt..Write$GT$10write_char17h32949ed49a1ffdbfE (type 2) (param i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32)
    local.get 0
    i32.load offset=8
    local.set 2
    block  ;; label = @1
      block  ;; label = @2
        local.get 1
        i32.const 128
        i32.ge_u
        br_if 0 (;@2;)
        i32.const 1
        local.set 3
        br 1 (;@1;)
      end
      block  ;; label = @2
        local.get 1
        i32.const 2048
        i32.ge_u
        br_if 0 (;@2;)
        i32.const 2
        local.set 3
        br 1 (;@1;)
      end
      i32.const 3
      i32.const 4
      local.get 1
      i32.const 65536
      i32.lt_u
      select
      local.set 3
    end
    local.get 2
    local.set 4
    block  ;; label = @1
      local.get 3
      local.get 0
      i32.load
      local.get 2
      i32.sub
      i32.le_u
      br_if 0 (;@1;)
      local.get 0
      local.get 2
      local.get 3
      i32.const 1
      i32.const 1
      call $_ZN5alloc7raw_vec20RawVecInner$LT$A$GT$7reserve21do_reserve_and_handle17hcef08ec0cc696d81E
      local.get 0
      i32.load offset=8
      local.set 4
    end
    local.get 0
    i32.load offset=4
    local.get 4
    i32.add
    local.set 4
    block  ;; label = @1
      block  ;; label = @2
        local.get 1
        i32.const 128
        i32.lt_u
        br_if 0 (;@2;)
        local.get 1
        i32.const 63
        i32.and
        i32.const -128
        i32.or
        local.set 5
        local.get 1
        i32.const 6
        i32.shr_u
        local.set 6
        block  ;; label = @3
          local.get 1
          i32.const 2048
          i32.ge_u
          br_if 0 (;@3;)
          local.get 4
          local.get 5
          i32.store8 offset=1
          local.get 4
          local.get 6
          i32.const 192
          i32.or
          i32.store8
          br 2 (;@1;)
        end
        local.get 1
        i32.const 12
        i32.shr_u
        local.set 7
        local.get 6
        i32.const 63
        i32.and
        i32.const -128
        i32.or
        local.set 6
        block  ;; label = @3
          local.get 1
          i32.const 65535
          i32.gt_u
          br_if 0 (;@3;)
          local.get 4
          local.get 5
          i32.store8 offset=2
          local.get 4
          local.get 6
          i32.store8 offset=1
          local.get 4
          local.get 7
          i32.const 224
          i32.or
          i32.store8
          br 2 (;@1;)
        end
        local.get 4
        local.get 5
        i32.store8 offset=3
        local.get 4
        local.get 6
        i32.store8 offset=2
        local.get 4
        local.get 7
        i32.const 63
        i32.and
        i32.const -128
        i32.or
        i32.store8 offset=1
        local.get 4
        local.get 1
        i32.const 18
        i32.shr_u
        i32.const -16
        i32.or
        i32.store8
        br 1 (;@1;)
      end
      local.get 4
      local.get 1
      i32.store8
    end
    local.get 0
    local.get 3
    local.get 2
    i32.add
    i32.store offset=8
    i32.const 0)
  (func $_ZN58_$LT$alloc..string..String$u20$as$u20$core..fmt..Write$GT$9write_str17he0f136bfd437b8d6E (type 1) (param i32 i32 i32) (result i32)
    (local i32)
    block  ;; label = @1
      local.get 2
      local.get 0
      i32.load
      local.get 0
      i32.load offset=8
      local.tee 3
      i32.sub
      i32.le_u
      br_if 0 (;@1;)
      local.get 0
      local.get 3
      local.get 2
      i32.const 1
      i32.const 1
      call $_ZN5alloc7raw_vec20RawVecInner$LT$A$GT$7reserve21do_reserve_and_handle17hcef08ec0cc696d81E
      local.get 0
      i32.load offset=8
      local.set 3
    end
    block  ;; label = @1
      local.get 2
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      i32.load offset=4
      local.get 3
      i32.add
      local.get 1
      local.get 2
      memory.copy
    end
    local.get 0
    local.get 3
    local.get 2
    i32.add
    i32.store offset=8
    i32.const 0)
  (func $_ZN86_$LT$std..panicking..panic_handler..StaticStrPayload$u20$as$u20$core..fmt..Display$GT$3fmt17hc8d087d6955a60e5E (type 2) (param i32 i32) (result i32)
    local.get 1
    local.get 0
    i32.load
    local.get 0
    i32.load offset=4
    call $_ZN4core3fmt9Formatter9write_str17h906c9016730dabacE)
  (func $_ZN89_$LT$std..panicking..panic_handler..FormatStringPayload$u20$as$u20$core..fmt..Display$GT$3fmt17he70713942f8152e4E (type 2) (param i32 i32) (result i32)
    block  ;; label = @1
      local.get 0
      i32.load
      i32.const -2147483648
      i32.eq
      br_if 0 (;@1;)
      local.get 1
      local.get 0
      i32.load offset=4
      local.get 0
      i32.load offset=8
      call $_ZN4core3fmt9Formatter9write_str17h906c9016730dabacE
      return
    end
    local.get 1
    i32.load
    local.get 1
    i32.load offset=4
    local.get 0
    i32.load offset=12
    i32.load
    local.tee 0
    i32.load
    local.get 0
    i32.load offset=4
    call $_ZN4core3fmt5write17h31474238f266a14aE)
  (func $_ZN8dlmalloc8dlmalloc17Dlmalloc$LT$A$GT$18insert_large_chunk17hc7cb11ca1345ed38E (type 0) (param i32 i32)
    (local i32 i32 i32 i32)
    i32.const 0
    local.set 2
    block  ;; label = @1
      local.get 1
      i32.const 256
      i32.lt_u
      br_if 0 (;@1;)
      i32.const 31
      local.set 2
      local.get 1
      i32.const 16777215
      i32.gt_u
      br_if 0 (;@1;)
      local.get 1
      i32.const 38
      local.get 1
      i32.const 8
      i32.shr_u
      i32.clz
      local.tee 2
      i32.sub
      i32.shr_u
      i32.const 1
      i32.and
      local.get 2
      i32.const 1
      i32.shl
      i32.sub
      i32.const 62
      i32.add
      local.set 2
    end
    local.get 0
    i64.const 0
    i64.store offset=16 align=4
    local.get 0
    local.get 2
    i32.store offset=28
    local.get 2
    i32.const 2
    i32.shl
    i32.const 1049136
    i32.add
    local.set 3
    block  ;; label = @1
      i32.const 0
      i32.load offset=1049548
      i32.const 1
      local.get 2
      i32.shl
      local.tee 4
      i32.and
      br_if 0 (;@1;)
      local.get 3
      local.get 0
      i32.store
      local.get 0
      local.get 3
      i32.store offset=24
      local.get 0
      local.get 0
      i32.store offset=12
      local.get 0
      local.get 0
      i32.store offset=8
      i32.const 0
      i32.const 0
      i32.load offset=1049548
      local.get 4
      i32.or
      i32.store offset=1049548
      return
    end
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          local.get 3
          i32.load
          local.tee 4
          i32.load offset=4
          i32.const -8
          i32.and
          local.get 1
          i32.ne
          br_if 0 (;@3;)
          local.get 4
          local.set 2
          br 1 (;@2;)
        end
        local.get 1
        i32.const 0
        i32.const 25
        local.get 2
        i32.const 1
        i32.shr_u
        i32.sub
        local.get 2
        i32.const 31
        i32.eq
        select
        i32.shl
        local.set 3
        loop  ;; label = @3
          local.get 4
          local.get 3
          i32.const 29
          i32.shr_u
          i32.const 4
          i32.and
          i32.add
          local.tee 5
          i32.load offset=16
          local.tee 2
          i32.eqz
          br_if 2 (;@1;)
          local.get 3
          i32.const 1
          i32.shl
          local.set 3
          local.get 2
          local.set 4
          local.get 2
          i32.load offset=4
          i32.const -8
          i32.and
          local.get 1
          i32.ne
          br_if 0 (;@3;)
        end
      end
      local.get 2
      i32.load offset=8
      local.tee 3
      local.get 0
      i32.store offset=12
      local.get 2
      local.get 0
      i32.store offset=8
      local.get 0
      i32.const 0
      i32.store offset=24
      local.get 0
      local.get 2
      i32.store offset=12
      local.get 0
      local.get 3
      i32.store offset=8
      return
    end
    local.get 5
    i32.const 16
    i32.add
    local.get 0
    i32.store
    local.get 0
    local.get 4
    i32.store offset=24
    local.get 0
    local.get 0
    i32.store offset=12
    local.get 0
    local.get 0
    i32.store offset=8)
  (func $_ZN93_$LT$std..panicking..panic_handler..StaticStrPayload$u20$as$u20$core..panic..PanicPayload$GT$3get17h1386487f02069651E (type 0) (param i32 i32)
    local.get 0
    i32.const 1048876
    i32.store offset=4
    local.get 0
    local.get 1
    i32.store)
  (func $_ZN93_$LT$std..panicking..panic_handler..StaticStrPayload$u20$as$u20$core..panic..PanicPayload$GT$6as_str17he125ac6f7c5f4796E (type 0) (param i32 i32)
    local.get 0
    local.get 1
    i64.load align=4
    i64.store)
  (func $_ZN93_$LT$std..panicking..panic_handler..StaticStrPayload$u20$as$u20$core..panic..PanicPayload$GT$8take_box17h0536373fffc709d7E (type 0) (param i32 i32)
    (local i32 i32)
    local.get 1
    i32.load offset=4
    local.set 2
    local.get 1
    i32.load
    local.set 3
    call $_RNvCs1Y7DaGC1cwg_7___rustc35___rust_no_alloc_shim_is_unstable_v2
    block  ;; label = @1
      i32.const 8
      i32.const 4
      call $_RNvCs1Y7DaGC1cwg_7___rustc12___rust_alloc
      local.tee 1
      br_if 0 (;@1;)
      i32.const 4
      i32.const 8
      call $_ZN5alloc5alloc18handle_alloc_error17hec8d3aa2a30efaa7E
      unreachable
    end
    local.get 1
    local.get 2
    i32.store offset=4
    local.get 1
    local.get 3
    i32.store
    local.get 0
    i32.const 1048876
    i32.store offset=4
    local.get 0
    local.get 1
    i32.store)
  (func $_ZN96_$LT$std..panicking..panic_handler..FormatStringPayload$u20$as$u20$core..panic..PanicPayload$GT$3get17heb4b8bac6804dc2bE (type 0) (param i32 i32)
    (local i32 i32 i32 i64)
    global.get $__stack_pointer
    i32.const 32
    i32.sub
    local.tee 2
    global.set $__stack_pointer
    block  ;; label = @1
      local.get 1
      i32.load
      i32.const -2147483648
      i32.ne
      br_if 0 (;@1;)
      local.get 1
      i32.load offset=12
      local.set 3
      local.get 2
      i32.const 20
      i32.add
      i32.const 8
      i32.add
      local.tee 4
      i32.const 0
      i32.store
      local.get 2
      i64.const 4294967296
      i64.store offset=20 align=4
      local.get 2
      i32.const 20
      i32.add
      i32.const 1048796
      local.get 3
      i32.load
      local.tee 3
      i32.load
      local.get 3
      i32.load offset=4
      call $_ZN4core3fmt5write17h31474238f266a14aE
      drop
      local.get 2
      i32.const 8
      i32.add
      i32.const 8
      i32.add
      local.get 4
      i32.load
      local.tee 3
      i32.store
      local.get 2
      local.get 2
      i64.load offset=20 align=4
      local.tee 5
      i64.store offset=8
      local.get 1
      i32.const 8
      i32.add
      local.get 3
      i32.store
      local.get 1
      local.get 5
      i64.store align=4
    end
    local.get 0
    i32.const 1049020
    i32.store offset=4
    local.get 0
    local.get 1
    i32.store
    local.get 2
    i32.const 32
    i32.add
    global.set $__stack_pointer)
  (func $_ZN96_$LT$std..panicking..panic_handler..FormatStringPayload$u20$as$u20$core..panic..PanicPayload$GT$8take_box17ha4d04ad03cee361aE (type 0) (param i32 i32)
    (local i32 i32 i32 i64)
    global.get $__stack_pointer
    i32.const 48
    i32.sub
    local.tee 2
    global.set $__stack_pointer
    block  ;; label = @1
      local.get 1
      i32.load
      i32.const -2147483648
      i32.ne
      br_if 0 (;@1;)
      local.get 1
      i32.load offset=12
      local.set 3
      local.get 2
      i32.const 36
      i32.add
      i32.const 8
      i32.add
      local.tee 4
      i32.const 0
      i32.store
      local.get 2
      i64.const 4294967296
      i64.store offset=36 align=4
      local.get 2
      i32.const 36
      i32.add
      i32.const 1048796
      local.get 3
      i32.load
      local.tee 3
      i32.load
      local.get 3
      i32.load offset=4
      call $_ZN4core3fmt5write17h31474238f266a14aE
      drop
      local.get 2
      i32.const 24
      i32.add
      i32.const 8
      i32.add
      local.get 4
      i32.load
      local.tee 3
      i32.store
      local.get 2
      local.get 2
      i64.load offset=36 align=4
      local.tee 5
      i64.store offset=24
      local.get 1
      i32.const 8
      i32.add
      local.get 3
      i32.store
      local.get 1
      local.get 5
      i64.store align=4
    end
    local.get 1
    i64.load align=4
    local.set 5
    local.get 1
    i64.const 4294967296
    i64.store align=4
    local.get 2
    i32.const 8
    i32.add
    i32.const 8
    i32.add
    local.tee 3
    local.get 1
    i32.const 8
    i32.add
    local.tee 1
    i32.load
    i32.store
    local.get 1
    i32.const 0
    i32.store
    local.get 2
    local.get 5
    i64.store offset=8
    call $_RNvCs1Y7DaGC1cwg_7___rustc35___rust_no_alloc_shim_is_unstable_v2
    block  ;; label = @1
      i32.const 12
      i32.const 4
      call $_RNvCs1Y7DaGC1cwg_7___rustc12___rust_alloc
      local.tee 1
      br_if 0 (;@1;)
      i32.const 4
      i32.const 12
      call $_ZN5alloc5alloc18handle_alloc_error17hec8d3aa2a30efaa7E
      unreachable
    end
    local.get 1
    local.get 2
    i64.load offset=8
    i64.store align=4
    local.get 1
    i32.const 8
    i32.add
    local.get 3
    i32.load
    i32.store
    local.get 0
    i32.const 1049020
    i32.store offset=4
    local.get 0
    local.get 1
    i32.store
    local.get 2
    i32.const 48
    i32.add
    global.set $__stack_pointer)
  (func $_ZN61_$LT$dlmalloc..sys..System$u20$as$u20$dlmalloc..Allocator$GT$5alloc17h5d11e6618597802fE (type 6) (param i32 i32 i32)
    (local i32 i32)
    block  ;; label = @1
      block  ;; label = @2
        local.get 2
        i32.const 16
        i32.shr_u
        local.get 2
        i32.const 65535
        i32.and
        i32.const 0
        i32.ne
        i32.add
        local.tee 2
        memory.grow
        local.tee 3
        i32.const -1
        i32.ne
        br_if 0 (;@2;)
        i32.const 0
        local.set 2
        i32.const 0
        local.set 4
        br 1 (;@1;)
      end
      local.get 2
      i32.const 16
      i32.shl
      local.tee 4
      i32.const -16
      i32.add
      local.get 4
      local.get 3
      i32.const 16
      i32.shl
      local.tee 2
      i32.const 0
      local.get 4
      i32.sub
      i32.eq
      select
      local.set 4
    end
    local.get 0
    i32.const 0
    i32.store offset=8
    local.get 0
    local.get 4
    i32.store offset=4
    local.get 0
    local.get 2
    i32.store)
  (func $_ZN5alloc5alloc18handle_alloc_error17hec8d3aa2a30efaa7E (type 0) (param i32 i32)
    local.get 1
    local.get 0
    call $_RNvCs1Y7DaGC1cwg_7___rustc26___rust_alloc_error_handler
    unreachable)
  (func $_ZN5alloc7raw_vec12handle_error17h9ace31a903e6893eE (type 0) (param i32 i32)
    block  ;; label = @1
      local.get 0
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      local.get 1
      call $_ZN5alloc5alloc18handle_alloc_error17hec8d3aa2a30efaa7E
      unreachable
    end
    call $_ZN5alloc7raw_vec17capacity_overflow17h0af0840ea1b2ff66E
    unreachable)
  (func $_ZN5alloc7raw_vec17capacity_overflow17h0af0840ea1b2ff66E (type 5)
    i32.const 1049036
    i32.const 35
    i32.const 1049056
    call $_ZN4core9panicking9panic_fmt17h6651313c3e2c6c2fE
    unreachable)
  (func $_ZN4core3fmt5write17h31474238f266a14aE (type 7) (param i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $__stack_pointer
    i32.const 16
    i32.sub
    local.tee 4
    global.set $__stack_pointer
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          local.get 3
          i32.const 1
          i32.and
          br_if 0 (;@3;)
          local.get 2
          i32.load8_u
          local.tee 5
          br_if 1 (;@2;)
          i32.const 0
          local.set 5
          br 2 (;@1;)
        end
        local.get 0
        local.get 2
        local.get 3
        i32.const 1
        i32.shr_u
        local.get 1
        i32.load offset=12
        call_indirect (type 1)
        local.set 5
        br 1 (;@1;)
      end
      local.get 1
      i32.load offset=12
      local.set 6
      i32.const 0
      local.set 7
      loop  ;; label = @2
        local.get 2
        i32.const 1
        i32.add
        local.set 8
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  local.get 5
                  i32.extend8_s
                  i32.const -1
                  i32.gt_s
                  br_if 0 (;@7;)
                  local.get 5
                  i32.const 255
                  i32.and
                  local.tee 9
                  i32.const 128
                  i32.eq
                  br_if 1 (;@6;)
                  local.get 9
                  i32.const 192
                  i32.ne
                  br_if 3 (;@4;)
                  local.get 4
                  local.get 1
                  i32.store offset=4
                  local.get 4
                  local.get 0
                  i32.store
                  local.get 4
                  i64.const 1610612768
                  i64.store offset=8 align=4
                  local.get 3
                  local.get 7
                  i32.const 3
                  i32.shl
                  i32.add
                  local.tee 5
                  i32.load
                  local.get 4
                  local.get 5
                  i32.load offset=4
                  call_indirect (type 2)
                  i32.eqz
                  br_if 2 (;@5;)
                  i32.const 1
                  local.set 5
                  br 6 (;@1;)
                end
                block  ;; label = @7
                  local.get 0
                  local.get 8
                  local.get 5
                  i32.const 255
                  i32.and
                  local.tee 5
                  local.get 6
                  call_indirect (type 1)
                  br_if 0 (;@7;)
                  local.get 8
                  local.get 5
                  i32.add
                  local.set 2
                  br 4 (;@3;)
                end
                i32.const 1
                local.set 5
                br 5 (;@1;)
              end
              block  ;; label = @6
                local.get 0
                local.get 2
                i32.const 3
                i32.add
                local.tee 5
                local.get 2
                i32.load16_u offset=1 align=1
                local.tee 2
                local.get 6
                call_indirect (type 1)
                br_if 0 (;@6;)
                local.get 5
                local.get 2
                i32.add
                local.set 2
                br 3 (;@3;)
              end
              i32.const 1
              local.set 5
              br 4 (;@1;)
            end
            local.get 7
            i32.const 1
            i32.add
            local.set 7
            local.get 8
            local.set 2
            br 1 (;@3;)
          end
          i32.const 1610612768
          local.set 10
          block  ;; label = @4
            local.get 5
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            local.get 2
            i32.const 5
            i32.add
            local.set 8
            local.get 2
            i32.load offset=1 align=1
            local.set 10
          end
          i32.const 0
          local.set 9
          block  ;; label = @4
            block  ;; label = @5
              local.get 5
              i32.const 2
              i32.and
              br_if 0 (;@5;)
              i32.const 0
              local.set 11
              local.get 8
              local.set 2
              br 1 (;@4;)
            end
            local.get 8
            i32.const 2
            i32.add
            local.set 2
            local.get 8
            i32.load16_u align=1
            local.set 11
          end
          block  ;; label = @4
            block  ;; label = @5
              local.get 5
              i32.const 4
              i32.and
              br_if 0 (;@5;)
              local.get 2
              local.set 8
              br 1 (;@4;)
            end
            local.get 2
            i32.const 2
            i32.add
            local.set 8
            local.get 2
            i32.load16_u align=1
            local.set 9
          end
          block  ;; label = @4
            block  ;; label = @5
              local.get 5
              i32.const 8
              i32.and
              br_if 0 (;@5;)
              local.get 8
              local.set 2
              br 1 (;@4;)
            end
            local.get 8
            i32.const 2
            i32.add
            local.set 2
            local.get 8
            i32.load16_u align=1
            local.set 7
          end
          block  ;; label = @4
            local.get 5
            i32.const 16
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            local.get 3
            local.get 11
            i32.const 65535
            i32.and
            i32.const 3
            i32.shl
            i32.add
            i32.load16_u offset=4
            local.set 11
          end
          block  ;; label = @4
            local.get 5
            i32.const 32
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            local.get 3
            local.get 9
            i32.const 65535
            i32.and
            i32.const 3
            i32.shl
            i32.add
            i32.load16_u offset=4
            local.set 9
          end
          local.get 4
          local.get 9
          i32.store16 offset=14
          local.get 4
          local.get 11
          i32.store16 offset=12
          local.get 4
          local.get 10
          i32.store offset=8
          local.get 4
          local.get 1
          i32.store offset=4
          local.get 4
          local.get 0
          i32.store
          block  ;; label = @4
            local.get 3
            local.get 7
            i32.const 3
            i32.shl
            i32.add
            local.tee 5
            i32.load
            local.get 4
            local.get 5
            i32.load offset=4
            call_indirect (type 2)
            i32.eqz
            br_if 0 (;@4;)
            i32.const 1
            local.set 5
            br 3 (;@1;)
          end
          local.get 7
          i32.const 1
          i32.add
          local.set 7
        end
        local.get 2
        i32.load8_u
        local.tee 5
        br_if 0 (;@2;)
      end
      i32.const 0
      local.set 5
    end
    local.get 4
    i32.const 16
    i32.add
    global.set $__stack_pointer
    local.get 5)
  (func $_ZN4core9panicking9panic_fmt17h6651313c3e2c6c2fE (type 6) (param i32 i32 i32)
    (local i32)
    global.get $__stack_pointer
    i32.const 32
    i32.sub
    local.tee 3
    global.set $__stack_pointer
    local.get 3
    local.get 1
    i32.store offset=16
    local.get 3
    local.get 0
    i32.store offset=12
    local.get 3
    i32.const 1
    i32.store16 offset=28
    local.get 3
    local.get 2
    i32.store offset=24
    local.get 3
    local.get 3
    i32.const 12
    i32.add
    i32.store offset=20
    local.get 3
    i32.const 20
    i32.add
    call $_RNvCs1Y7DaGC1cwg_7___rustc17rust_begin_unwind
    unreachable)
  (func $_ZN4core9panicking5panic17h0149fc8f1656305aE (type 6) (param i32 i32 i32)
    local.get 0
    local.get 1
    i32.const 1
    i32.shl
    i32.const 1
    i32.or
    local.get 2
    call $_ZN4core9panicking9panic_fmt17h6651313c3e2c6c2fE
    unreachable)
  (func $_ZN4core3fmt9Formatter9write_str17h906c9016730dabacE (type 1) (param i32 i32 i32) (result i32)
    local.get 0
    i32.load
    local.get 1
    local.get 2
    local.get 0
    i32.load offset=4
    i32.load offset=12
    call_indirect (type 1))
  (func $_ZN4core9panicking11panic_const23panic_const_rem_by_zero17h4d91c9c4a6b3b2e4E (type 8) (param i32)
    i32.const 1049072
    i32.const 115
    local.get 0
    call $_ZN4core9panicking9panic_fmt17h6651313c3e2c6c2fE
    unreachable)
  (table (;0;) 17 17 funcref)
  (memory (;0;) 17)
  (global $__stack_pointer (mut i32) (i32.const 1048576))
  (global (;1;) i32 (i32.const 1049620))
  (global (;2;) i32 (i32.const 1049632))
  (export "memory" (memory 0))
  (export "add" (func $add))
  (export "factorial" (func $factorial))
  (export "fibonacci" (func $fibonacci))
  (export "get_counter" (func $get_counter))
  (export "increment" (func $increment))
  (export "is_prime" (func $is_prime))
  (export "reset_counter" (func $reset_counter))
  (export "__data_end" (global 1))
  (export "__heap_base" (global 2))
  (elem (;0;) (i32.const 1) func $_ZN3std5alloc24default_alloc_error_hook17h4ae318a4060b4b2eE $_ZN4core3ptr42drop_in_place$LT$alloc..string..String$GT$17h5f918006ef5d0ce0E $_ZN58_$LT$alloc..string..String$u20$as$u20$core..fmt..Write$GT$9write_str17he0f136bfd437b8d6E $_ZN58_$LT$alloc..string..String$u20$as$u20$core..fmt..Write$GT$10write_char17h32949ed49a1ffdbfE $_ZN4core3fmt5Write9write_fmt17h628b111ce4addafbE $_ZN86_$LT$std..panicking..panic_handler..StaticStrPayload$u20$as$u20$core..fmt..Display$GT$3fmt17hc8d087d6955a60e5E $_ZN93_$LT$std..panicking..panic_handler..StaticStrPayload$u20$as$u20$core..panic..PanicPayload$GT$8take_box17h0536373fffc709d7E $_ZN93_$LT$std..panicking..panic_handler..StaticStrPayload$u20$as$u20$core..panic..PanicPayload$GT$3get17h1386487f02069651E $_ZN93_$LT$std..panicking..panic_handler..StaticStrPayload$u20$as$u20$core..panic..PanicPayload$GT$6as_str17he125ac6f7c5f4796E $_ZN4core3ptr71drop_in_place$LT$std..panicking..panic_handler..FormatStringPayload$GT$17hf7b8d0e0fb4c83c3E $_ZN89_$LT$std..panicking..panic_handler..FormatStringPayload$u20$as$u20$core..fmt..Display$GT$3fmt17he70713942f8152e4E $_ZN96_$LT$std..panicking..panic_handler..FormatStringPayload$u20$as$u20$core..panic..PanicPayload$GT$8take_box17ha4d04ad03cee361aE $_ZN96_$LT$std..panicking..panic_handler..FormatStringPayload$u20$as$u20$core..panic..PanicPayload$GT$3get17heb4b8bac6804dc2bE $_ZN4core5panic12PanicPayload6as_str17h17c1d5120cb39a3fE $_ZN36_$LT$T$u20$as$u20$core..any..Any$GT$7type_id17h054965a0d695c3aeE $_ZN36_$LT$T$u20$as$u20$core..any..Any$GT$7type_id17h81f53a1fd46151f1E)
  (data $.rodata (i32.const 1048576) "/rustc/e408947bfd200af42db322daf0fadfe7e26d3bd1/library/alloc/src/raw_vec/mod.rs\00/rust/deps/dlmalloc-0.2.11/src/dlmalloc.rs\00crates/example-wasm/src/lib.rs\00\00|\00\10\00\1e\00\00\00%\00\00\00\0c\00\00\00|\00\10\00\1e\00\00\00%\00\00\00\1a\00\00\00m]\cb\d6,P\ebcxA\a6Wq\1b\8b\b9\9a\b8\ee\91Q\14\96X\cf\96\00\e8\d2\9f\12\8a\02\00\00\00\0c\00\00\00\04\00\00\00\03\00\00\00\04\00\00\00\05\00\00\00\00\00\00\00\08\00\00\00\04\00\00\00\06\00\00\00\07\00\00\00\08\00\00\00\09\00\00\00\0a\00\00\00\10\00\00\00\04\00\00\00\0b\00\00\00\0c\00\00\00\0d\00\00\00\0e\00\00\00\00\00\00\00\08\00\00\00\04\00\00\00\0f\00\00\00assertion failed: psize >= size + min_overhead\00\00Q\00\10\00*\00\00\00\b1\04\00\00\09\00\00\00assertion failed: psize <= size + max_overhead\00\00Q\00\10\00*\00\00\00\b7\04\00\00\0d\00\00\00\02\00\00\00\0c\00\00\00\04\00\00\00\10\00\00\00capacity overflow\00\00\00\00\00\10\00P\00\00\00\1c\00\00\00\05\00\00\00attempt to calculate the remainder with a divisor of zero"))
