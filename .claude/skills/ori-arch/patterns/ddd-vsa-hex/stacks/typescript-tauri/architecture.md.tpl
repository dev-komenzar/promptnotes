---
version: 1
default_root: ts
workspace:
  apps_root: apps
  apps:
    - name: {{APP_NAME}}
      path: apps/{{APP_NAME}}
roots:
  - id: ts
    app: {{APP_NAME}}
    path: apps/{{APP_NAME}}/src
    language: typescript
    layer_set: ddd-vsa-hex-ts
    adapter: eslint
    slice_root: {{BC_NAME}}
    slice_subdir: slices
    public_entry: index.ts
  - id: rs
    app: {{APP_NAME}}
    path: apps/{{APP_NAME}}/src-tauri/src
    language: rust
    layer_set: ddd-vsa-hex-rs
    adapter: rust
    slice_root: {{BC_NAME_RS}}
    slice_subdir: slices
    public_entry: mod.rs
cross_root:
  - from: { root: rs, path: apps/{{APP_NAME}}/src-tauri/src/{{BC_NAME_RS}}/slices/<slice_rs>/commands.rs }
    to:   { root: ts, path: apps/{{APP_NAME}}/src/{{BC_NAME}}/shared/ipc/bindings.ts }
    generator: tauri-specta
    auto_generated: true
layer_sets:
  ddd-vsa-hex-ts:
    layers:
      - { id: shared,    kind: shared }
      - { id: domain,    kind: slice, slice_internal: slice-internal-ts }
      - { id: ui-widget, kind: ui-layer, order: 1 }
      - { id: ui-page,   kind: ui-layer, order: 2 }
    rules:
      cross_layer:
        - { from: ui-page,   allow: [ui-widget, shared, domain] }
        - { from: ui-widget, allow: [shared, domain] }
        - { from: domain,    allow: [shared] }
        - { from: shared,    allow: [] }
      same_layer: prohibited
      public_entry_required: true
      forbidden_imports:
        - from: ui-widget
          modules: ["@tauri-apps/api/core"]
          reason: "use {{BC_NAME}}/shared/ipc/* (tauri-specta-generated bindings) instead of raw invoke"
        - from: ui-page
          modules: ["@tauri-apps/api/core"]
          reason: "use {{BC_NAME}}/shared/ipc/* (tauri-specta-generated bindings) instead of raw invoke"
  ddd-vsa-hex-rs:
    layers:
      - { id: shared, kind: shared }
      - { id: domain, kind: slice, slice_internal: slice-internal-rs }
    rules:
      cross_layer:
        - { from: domain, allow: [shared] }
        - { from: shared, allow: [] }
      same_layer: prohibited
      public_entry_required: true
slice_internal:
  slice-internal-ts:
    sub_layers: [domain, application, infrastructure, presentation, tests]
    rules:
      - { from: presentation,   allow: [application, domain] }
      - { from: application,    allow: [domain] }
      - { from: infrastructure, allow: [domain] }
      - { from: domain,         allow: [] }
      - { from: tests,          allow: [domain, application, infrastructure, presentation] }
  slice-internal-rs:
    sub_layers: [domain, application, infrastructure, presentation]
    rules:
      - { from: presentation,   allow: [application, domain] }
      - { from: application,    allow: [domain, infrastructure] }
      - { from: infrastructure, allow: [domain] }
      - { from: domain,         allow: [] }
cross_slice:
  prohibited_direct: true
  via: [shared/contracts, shared/events]
cross_bc:
  via: [apps/{{APP_NAME}}/src/shared/contracts, apps/{{APP_NAME}}/src/shared/events]
  same_event_bus: true
phase_hooks:
  flow-impl-red-pre:
    - cmd: cargo run --bin export-types
      cwd: apps/{{APP_NAME}}/src-tauri
      reason: "regenerate tauri-specta bindings before authoring red boundary tests"
  flow-impl-green-post:
    - cmd: cargo run --bin export-types
      cwd: apps/{{APP_NAME}}/src-tauri
      reason: "resync TS bindings after rust impl changes (DoD rule 4)"
---

# Architecture ({{APP_NAME}} — ddd-vsa-hex / typescript-tauri)

This file is the **single source of truth** for both the TypeScript frontend
(`apps/{{APP_NAME}}/src/`) and the Rust backend
(`apps/{{APP_NAME}}/src-tauri/src/`). Two adapters compile it:

```bash
# TypeScript root (default)
node .apm/skills/ori-arch/scripts/export.js --adapter=eslint --root=ts
# Rust root
node .apm/skills/ori-arch/scripts/export.js --adapter=rust --root=rs
```

## Roots

| id  | path                                        | language    | adapter | slice_root        | slice_subdir | public_entry |
| --- | ------------------------------------------- | ----------- | ------- | ----------------- | ------------ | ------------ |
| ts  | `apps/{{APP_NAME}}/src`                     | typescript  | eslint  | `{{BC_NAME}}`     | `slices`     | `index.ts`   |
| rs  | `apps/{{APP_NAME}}/src-tauri/src`           | rust        | rust    | `{{BC_NAME_RS}}`  | `slices`     | `mod.rs`     |

The two roots are bridged by **tauri-specta**, which derives the TS bindings
under `apps/{{APP_NAME}}/src/{{BC_NAME}}/shared/ipc/bindings.ts` from
the `#[tauri::command]` functions in
`apps/{{APP_NAME}}/src-tauri/src/{{BC_NAME_RS}}/slices/<slice_rs>/commands.rs`.
This is the only sanctioned cross-root contract; everything else stays inside
its own root. (Rust identifier rules require underscores, hence the
`{{BC_NAME_RS}}` spelling on the Rust side.)

See `.apm/skills/ori-arch/patterns/ddd-vsa-hex/stacks/typescript-tauri/example-slice/`
for a worked slice (`complete-task` on the TS side, `complete_task` on the
Rust side) — AI agents read it on demand when generating new slices.

## Layout (TypeScript)

```
apps/{{APP_NAME}}/src/
├── {{BC_NAME}}/
│   ├── shared/                # BC-internal shared (kind: shared)
│   │   ├── ipc/               # tauri-specta-generated bindings (regenerated on build)
│   │   ├── types/
│   │   ├── events/
│   │   └── contracts/
│   └── slices/
│       └── <slice-id>/        # 1 slice per use case
│           ├── index.ts       # PUBLIC API
│           ├── domain/
│           ├── application/
│           ├── infrastructure/
│           ├── presentation/
│           └── tests/
├── ui-widget/                 # ddd-vsa-hex ui-layer (order 1)
└── ui-page/                   # ddd-vsa-hex ui-layer (order 2)
```

## Layout (Rust)

```
apps/{{APP_NAME}}/src-tauri/src/
├── lib.rs                       # crate root; declares `pub mod {{BC_NAME_RS}}`
├── main.rs                      # binary entry (delegated to `tauri init`)
└── {{BC_NAME_RS}}/
    ├── mod.rs                   # `pub mod shared; pub mod slices;`
    ├── shared/                  # below every slice in the dependency graph
    │   ├── mod.rs               # PUBLIC API
    │   ├── result.rs            # AppError / AppResult
    │   └── events.rs            # DomainEvent
    └── slices/
        ├── mod.rs               # `pub mod <slice_rs>;`
        └── <slice_rs>/          # one folder per backend slice
            ├── mod.rs           # PUBLIC API — only this file is `pub use`d outside
            ├── domain.rs
            ├── application.rs
            ├── infrastructure.rs
            └── commands.rs      # tauri-specta surface
```

## Rules

### Shared (both sides)

- **Cross-slice direct imports are prohibited.** Use
  `{{BC_NAME}}/shared/contracts/` (TS) / `{{BC_NAME_RS}}::shared`
  (Rust) or domain events to collaborate across slices.
- **Each slice has exactly one public entry**: `index.ts` (TS) / `mod.rs`
  (Rust).

### TypeScript-specific

- UI layers form a one-way pipeline `ui-page -> ui-widget -> {shared,
  domain}`. Same-layer imports are prohibited.
- **No raw `@tauri-apps/api/core` imports from any UI layer.** Use the
  tauri-specta-generated bindings under `{{BC_NAME}}/shared/ipc/`. The
  eslint adapter emits a `no-restricted-imports` rule that fails the build
  on raw `invoke` calls — sourced from `forbidden_imports` above.

### Rust-specific

- The arch-adapter-rust enforces cross-slice and cross-layer rules by
  walking `use` statements. `crate::*`, `super::*`, and `self::*` are all
  resolved against the Rust 2018+ module-file convention.
- Cross-slice direct imports (e.g.,
  `crate::{{BC_NAME_RS}}::slices::other` from inside
  `{{BC_NAME_RS}}::slices::<slice_rs>`) are rejected by the generated
  `tests/arch.rs`.

## Test Contract (typescript-tauri instantiation)

A slice's DoD test MUST:

- Live under `apps/{{APP_NAME}}/src/{{BC_NAME}}/slices/<slice-id>/tests/`.
- Import the command surface only via
  `import { commands } from "{{BC_NAME}}/shared/ipc/bindings";`
  (auto-generated by tauri-specta from the slice's `commands.rs`).
- Construct the slice with the production wiring:
  ```ts
  import { mockIPC } from "@tauri-apps/api/mocks";
  import { setupProductionBuilder } from "{{BC_NAME}}/shared/test-fixtures";
  beforeEach(() => mockIPC(setupProductionBuilder()));
  ```

Phase hooks (auto-installed by /ori-init):

- flow-impl-red-pre:    cargo run --bin export-types
- flow-impl-green-post: cargo run --bin export-types

These are declared in the frontmatter `phase_hooks` block above and are
re-emitted by `/ori-arch` on each regeneration; `/ori-flow` reads them
to invoke the binding regeneration before red tests are authored and
after green impl is committed (DoD rule 4 in `pattern.md`).

Regenerate after editing this file:

```bash
node .apm/skills/ori-arch/scripts/export.js --adapter=eslint --root=ts
node .apm/skills/ori-arch/scripts/export.js --adapter=rust   --root=rs
```
