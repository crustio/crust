// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use substrate_wasm_builder::WasmBuilder;

fn main() {
    WasmBuilder::new()
        .with_current_project()
        .import_memory()
        .export_heap_base()
        .build()
}

