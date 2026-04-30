1. **Add `IntoIterator` for `&mut RedArray`** in `src/types/array/mod.rs`.
2. **Add `iter_mut`, `pop` methods** in `src/types/array/mod.rs` for `RedArray`.
3. **Add documentations** to existing methods in `src/types/array/static_array.rs`, `src/types/array/mod.rs`, and `src/types/hash.rs`.
4. **Implement traits and methods for `StaticArray`** in `src/types/array/static_array.rs`:
   - `len`, `is_empty`, `capacity`
   - `Deref`, `DerefMut` to `[T]`
   - `AsRef<[T]>`, `AsMut<[T]>`
   - `IntoIterator` for `&StaticArray`, `&mut StaticArray`
5. **Implement traits and methods for `RedHashMap`** in `src/types/hash.rs`:
   - `len`, `is_empty`, `clear`, `remove`
   - `keys`, `values`, `iter_mut`
   - `IntoIterator` for `&mut RedHashMap` and `RedHashMap` (Wait, RedHashMap owns elements? Yes, implementing IntoIter for RedHashMap requires a custom iterator).
   - `Drop` for `RedHashMap` to free `nodeList.nodes` and drop keys/values.
6. **Pre-commit checks** using `pre_commit_instructions` tool.
