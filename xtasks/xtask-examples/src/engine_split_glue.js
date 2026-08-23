export function engine_memory() { return wasm.memory; }
export function engine_function_table() { return wasm.__indirect_function_table; }
export function raw_engine_alloc(size, align) { return wasm.engine_alloc(size, align); }
export function raw_engine_alloc_zeroed(size, align) { return wasm.engine_alloc_zeroed(size, align); }
export function raw_engine_dealloc(ptr, size, align) { wasm.engine_dealloc(ptr, size, align); }
export function raw_engine_realloc(ptr, old_size, new_size, align) { return wasm.engine_realloc(ptr, old_size, new_size, align); }
