
/// @use-src 0:"test.sol"
object "Vault_44" {
    code {
        /// @src 0:58:401
        mstore(64, memoryguard(128))
        if callvalue() { revert_error_ca66f745a3ce8ff40e2ccaf1ad45db7774001b90d25810abd9040049be7bf4bb() }

        constructor_Vault_44()

        let _1 := allocate_unbounded()
        codecopy(_1, dataoffset("Vault_44_deployed"), datasize("Vault_44_deployed"))

        return(_1, datasize("Vault_44_deployed"))

        function allocate_unbounded() -> memPtr {
            memPtr := mload(64)
        }

        function revert_error_ca66f745a3ce8ff40e2ccaf1ad45db7774001b90d25810abd9040049be7bf4bb() {
            revert(0, 0)
        }

        /// @src 0:58:401
        function constructor_Vault_44() {

            /// @src 0:58:401

        }
        /// @src 0:58:401

    }
    /// @use-src 0:"test.sol"
    object "Vault_44_deployed" {
        code {
            /// @src 0:58:401
            mstore(64, memoryguard(128))

            if iszero(lt(calldatasize(), 4))
            {
                let selector := shift_right_224_unsigned(calldataload(0))
                switch selector

                case 0x2ddbd13a
                {
                    // total()

                    external_fun_total_10()
                }

                case 0xb6b55f25
                {
                    // deposit(uint256)

                    external_fun_deposit_43()
                }

                default {}
            }

            revert_error_42b3090547df1d2001c96683413b8cf91c1b902ef5e3cb8d9f6f304cf7446f74()

            function shift_right_224_unsigned(value) -> newValue {
                newValue :=

                shr(224, value)

            }

            function allocate_unbounded() -> memPtr {
                memPtr := mload(64)
            }

            function revert_error_ca66f745a3ce8ff40e2ccaf1ad45db7774001b90d25810abd9040049be7bf4bb() {
                revert(0, 0)
            }

            function revert_error_dbdddcbe895c83990c08b3492a0e83918d802a52331272ac6fdb6a7c4aea3b1b() {
                revert(0, 0)
            }

            function abi_decode_tuple_(headStart, dataEnd)   {
                if slt(sub(dataEnd, headStart), 0) { revert_error_dbdddcbe895c83990c08b3492a0e83918d802a52331272ac6fdb6a7c4aea3b1b() }

            }

            function shift_right_unsigned_dynamic(bits, value) -> newValue {
                newValue :=

                shr(bits, value)

            }

            function cleanup_from_storage_t_uint256(value) -> cleaned {
                cleaned := value
            }

            function extract_from_storage_value_dynamict_uint256(slot_value, offset) -> value {
                value := cleanup_from_storage_t_uint256(shift_right_unsigned_dynamic(mul(offset, 8), slot_value))
            }

            function read_from_storage_split_dynamic_t_uint256(slot, offset) -> value {
                value := extract_from_storage_value_dynamict_uint256(sload(slot), offset)

            }

            /// @ast-id 10
            /// @src 0:161:181
            function getter_fun_total_10() -> ret {

                let slot := 1
                let offset := 0

                ret := read_from_storage_split_dynamic_t_uint256(slot, offset)

            }
            /// @src 0:58:401

            function cleanup_t_uint256(value) -> cleaned {
                cleaned := value
            }

            function abi_encode_t_uint256_to_t_uint256_fromStack(value, pos) {
                mstore(pos, cleanup_t_uint256(value))
            }

            function abi_encode_tuple_t_uint256__to_t_uint256__fromStack(headStart , value0) -> tail {
                tail := add(headStart, 32)

                abi_encode_t_uint256_to_t_uint256_fromStack(value0,  add(headStart, 0))

            }

            function external_fun_total_10() {

                if callvalue() { revert_error_ca66f745a3ce8ff40e2ccaf1ad45db7774001b90d25810abd9040049be7bf4bb() }
                abi_decode_tuple_(4, calldatasize())
                let ret_0 :=  getter_fun_total_10()
                let memPos := allocate_unbounded()
                let memEnd := abi_encode_tuple_t_uint256__to_t_uint256__fromStack(memPos , ret_0)
                return(memPos, sub(memEnd, memPos))

            }

            function revert_error_c1322bf8034eace5e0b5c7295db60986aa89aae5e0ea0873e4689e076861a5db() {
                revert(0, 0)
            }

            function validator_revert_t_uint256(value) {
                if iszero(eq(value, cleanup_t_uint256(value))) { revert(0, 0) }
            }

            function abi_decode_t_uint256(offset, end) -> value {
                value := calldataload(offset)
                validator_revert_t_uint256(value)
            }

            function abi_decode_tuple_t_uint256(headStart, dataEnd) -> value0 {
                if slt(sub(dataEnd, headStart), 32) { revert_error_dbdddcbe895c83990c08b3492a0e83918d802a52331272ac6fdb6a7c4aea3b1b() }

                {

                    let offset := 0

                    value0 := abi_decode_t_uint256(add(headStart, offset), dataEnd)
                }

            }

            function abi_encode_tuple__to__fromStack(headStart ) -> tail {
                tail := add(headStart, 0)

            }

            function external_fun_deposit_43() {

                if callvalue() { revert_error_ca66f745a3ce8ff40e2ccaf1ad45db7774001b90d25810abd9040049be7bf4bb() }
                let param_0 :=  abi_decode_tuple_t_uint256(4, calldatasize())
                fun_deposit_43(param_0)
                let memPos := allocate_unbounded()
                let memEnd := abi_encode_tuple__to__fromStack(memPos  )
                return(memPos, sub(memEnd, memPos))

            }

            function revert_error_42b3090547df1d2001c96683413b8cf91c1b902ef5e3cb8d9f6f304cf7446f74() {
                revert(0, 0)
            }

            function cleanup_t_rational_100_by_1(value) -> cleaned {
                cleaned := value
            }

            function identity(value) -> ret {
                ret := value
            }

            function convert_t_rational_100_by_1_to_t_uint256(value) -> converted {
                converted := cleanup_t_uint256(identity(cleanup_t_rational_100_by_1(value)))
            }

            /// @src 0:79:105
            function constant_MAX_4() -> ret {
                /// @src 0:102:105
                let expr_3 := 0x64
                let _2 := convert_t_rational_100_by_1_to_t_uint256(expr_3)

                ret := _2
            }

            function array_storeLengthForEncoding_t_string_memory_ptr_fromStack(pos, length) -> updated_pos {
                mstore(pos, length)
                updated_pos := add(pos, 0x20)
            }

            function store_literal_in_memory_02657998c17889bca23b1c61920eb7e27aa5a711953d8fcc2f68716e8028827d(memPtr) {

                mstore(add(memPtr, 0), "too much")

            }

            function abi_encode_t_stringliteral_02657998c17889bca23b1c61920eb7e27aa5a711953d8fcc2f68716e8028827d_to_t_string_memory_ptr_fromStack(pos) -> end {
                pos := array_storeLengthForEncoding_t_string_memory_ptr_fromStack(pos, 8)
                store_literal_in_memory_02657998c17889bca23b1c61920eb7e27aa5a711953d8fcc2f68716e8028827d(pos)
                end := add(pos, 32)
            }

            function abi_encode_tuple_t_stringliteral_02657998c17889bca23b1c61920eb7e27aa5a711953d8fcc2f68716e8028827d__to_t_string_memory_ptr__fromStack(headStart ) -> tail {
                tail := add(headStart, 32)

                mstore(add(headStart, 0), sub(tail, headStart))
                tail := abi_encode_t_stringliteral_02657998c17889bca23b1c61920eb7e27aa5a711953d8fcc2f68716e8028827d_to_t_string_memory_ptr_fromStack( tail)

            }

            function require_helper_t_stringliteral_02657998c17889bca23b1c61920eb7e27aa5a711953d8fcc2f68716e8028827d(condition ) {
                if iszero(condition) {
                    let memPtr := allocate_unbounded()
                    mstore(memPtr, 0x08c379a000000000000000000000000000000000000000000000000000000000)
                    let end := abi_encode_tuple_t_stringliteral_02657998c17889bca23b1c61920eb7e27aa5a711953d8fcc2f68716e8028827d__to_t_string_memory_ptr__fromStack(add(memPtr, 4) )
                    revert(memPtr, sub(end, memPtr))
                }
            }

            function cleanup_t_uint160(value) -> cleaned {
                cleaned := and(value, 0xffffffffffffffffffffffffffffffffffffffff)
            }

            function convert_t_uint160_to_t_uint160(value) -> converted {
                converted := cleanup_t_uint160(identity(cleanup_t_uint160(value)))
            }

            function convert_t_uint160_to_t_address(value) -> converted {
                converted := convert_t_uint160_to_t_uint160(value)
            }

            function convert_t_address_to_t_address(value) -> converted {
                converted := convert_t_uint160_to_t_address(value)
            }

            function mapping_index_access_t_mapping$_t_address_$_t_uint256_$_of_t_address(slot , key) -> dataSlot {
                mstore(0, convert_t_address_to_t_address(key))
                mstore(0x20, slot)
                dataSlot := keccak256(0, 0x40)
            }

            function shift_right_0_unsigned(value) -> newValue {
                newValue :=

                shr(0, value)

            }

            function extract_from_storage_value_offset_0t_uint256(slot_value) -> value {
                value := cleanup_from_storage_t_uint256(shift_right_0_unsigned(slot_value))
            }

            function read_from_storage_split_offset_0_t_uint256(slot) -> value {
                value := extract_from_storage_value_offset_0t_uint256(sload(slot))

            }

            function panic_error_0x11() {
                mstore(0, 35408467139433450592217433187231851964531694900788300625387963629091585785856)
                mstore(4, 0x11)
                revert(0, 0x24)
            }

            function checked_add_t_uint256(x, y) -> sum {
                x := cleanup_t_uint256(x)
                y := cleanup_t_uint256(y)
                sum := add(x, y)

                if gt(x, sum) { panic_error_0x11() }

            }

            function shift_left_0(value) -> newValue {
                newValue :=

                shl(0, value)

            }

            function update_byte_slice_32_shift_0(value, toInsert) -> result {
                let mask := 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
                toInsert := shift_left_0(toInsert)
                value := and(value, not(mask))
                result := or(value, and(toInsert, mask))
            }

            function convert_t_uint256_to_t_uint256(value) -> converted {
                converted := cleanup_t_uint256(identity(cleanup_t_uint256(value)))
            }

            function prepare_store_t_uint256(value) -> ret {
                ret := value
            }

            function update_storage_value_offset_0t_uint256_to_t_uint256(slot, value_0) {
                let convertedValue_0 := convert_t_uint256_to_t_uint256(value_0)
                sstore(slot, update_byte_slice_32_shift_0(sload(slot), prepare_store_t_uint256(convertedValue_0)))
            }

            /// @ast-id 43
            /// @src 0:188:399
            function fun_deposit_43(var_amount_12) {

                /// @src 0:246:252
                let _1 := var_amount_12
                let expr_16 := _1
                /// @src 0:256:259
                let expr_17 := constant_MAX_4()
                /// @src 0:246:259
                let expr_18 := iszero(gt(cleanup_t_uint256(expr_16), cleanup_t_uint256(expr_17)))
                /// @src 0:238:272
                require_helper_t_stringliteral_02657998c17889bca23b1c61920eb7e27aa5a711953d8fcc2f68716e8028827d(expr_18)
                /// @src 0:299:307
                let _3_slot := 0x00
                let expr_24_slot := _3_slot
                /// @src 0:308:318
                let expr_26 := caller()
                /// @src 0:299:319
                let _4 := mapping_index_access_t_mapping$_t_address_$_t_uint256_$_of_t_address(expr_24_slot,expr_26)
                let _5 := read_from_storage_split_offset_0_t_uint256(_4)
                let expr_27 := _5
                /// @src 0:322:328
                let _6 := var_amount_12
                let expr_28 := _6
                /// @src 0:299:328
                let expr_29 := checked_add_t_uint256(expr_27, expr_28)

                /// @src 0:282:328
                let var_newBal_23 := expr_29
                /// @src 0:361:367
                let _7 := var_newBal_23
                let expr_35 := _7
                /// @src 0:338:346
                let _8_slot := 0x00
                let expr_31_slot := _8_slot
                /// @src 0:347:357
                let expr_33 := caller()
                /// @src 0:338:358
                let _9 := mapping_index_access_t_mapping$_t_address_$_t_uint256_$_of_t_address(expr_31_slot,expr_33)
                /// @src 0:338:367
                update_storage_value_offset_0t_uint256_to_t_uint256(_9, expr_35)
                let expr_36 := expr_35
                /// @src 0:386:392
                let _10 := var_amount_12
                let expr_39 := _10
                /// @src 0:377:392
                let _11 := read_from_storage_split_offset_0_t_uint256(0x01)
                let expr_40 := checked_add_t_uint256(_11, expr_39)

                update_storage_value_offset_0t_uint256_to_t_uint256(0x01, expr_40)

            }
            /// @src 0:58:401

        }

        data ".metadata" hex"a2646970667358221220b4569ff6b5b1e2b661de16ff9ebd81da5514575c8030661d3895f32398bd0cac64736f6c63430008180033"
    }

}

