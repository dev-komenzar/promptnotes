// Specta type export entry for tauri-specta.
//
// Invoked by:
//   - `cargo run --bin export-types` from src-tauri/
//   - apm-scripts/specta-build.sh
//   - the `phase_hooks.flow-impl-{red-pre,green-post}` declared in
//     .ori/architecture.md (frontmatter) — see Slice DoD rule 4 in
//     .apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md
//
// Output: ../src/__BC_NAME__/shared/ipc/bindings.ts
//
// The bindings.ts is the *single* cross-root contract surface for the
// __BC_NAME__ BC. Hand-editing it is prohibited (the next specta run
// will overwrite the file).

use specta_typescript::Typescript;
use tauri_specta::{collect_commands, Builder};

// Aggregate each slice's command surface here. When a new slice ships,
// add a `pub use` to the slice's `commands` module above the macro
// call and append the command identifier inside `collect_commands![]`.
//
// Example:
//   use __APP_NAME_RS__::__BC_NAME_RS__::slices::complete_task::commands::complete_task_cmd;
//   ...
//   .commands(collect_commands![complete_task_cmd]);

fn main() {
    let builder = Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            // append per-slice commands here
        ]);

    builder
        .export(
            Typescript::default(),
            "../src/__BC_NAME__/shared/ipc/bindings.ts",
        )
        .expect("failed to export tauri-specta bindings");
}
