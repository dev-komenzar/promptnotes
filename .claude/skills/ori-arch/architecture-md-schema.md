# `.ori/architecture.md` schema (v1)

> Cross-skill 共有 SSoT。`.ori/architecture.md` (per-project file)の machine-readable
> contract を定義する。3 つの contract を支える:
> 1. `/ori-arch`(生成側) ↔ `.ori/architecture.md`
> 2. `.ori/architecture.md` ↔ Adapters(消費側)
> 3. Schema 自身の version 管理(v1, v2, ...)

`.ori/architecture.md` is the **language-neutral SSoT** for ddd-vsa-hex architecture
enforcement. It declares layers, allowed cross-layer dependencies, and public entry
points. Adapters compile it to native linter configs (eslint, dependency-cruiser,
import-linter, rust integration test, generic regex fallback).

The file is **YAML frontmatter + Markdown body**. Frontmatter is the machine-parseable
contract; body is rationale, examples, and human-maintained notes (preserved across
auto-regeneration).

This schema is `version: 1`. v0.1 implementations MUST accept the multi-root extension
fields even if they only execute single-root projects; unknown root configurations
should fail loudly rather than be silently ignored.

---

## Top-level frontmatter

```yaml
---
version: 1                    # schema version; required
workspace:                    # apps directory 構成(/ori-init が repo folder 名から自動導出)
  apps_root: apps             # project root から見た apps directory
  apps:
    - name: <project-folder>  # /ori-init が repo folder 名を sanitize して導出
      path: apps/<project-folder>
default_root: ts              # optional. used when 'roots' is absent。値は roots[].id
roots:                        # optional in v0.1; required for multi-root projects
  - id: ts
    app: <project-folder>     # この root が属する app(workspace.apps の name と一致)
    path: apps/<project-folder>/src
    language: typescript
    layer_set: ddd-vsa-hex-ts
    adapter: eslint           # adapter ID (eslint | rust | generic | dependency-cruiser | import-linter)
    slice_root: <bc>          # BC namespace directory (one level under <path>)
    slice_subdir: slices      # optional. nested dir under slice_root holding slices (design.md §17)
    public_entry: index.ts    # file that exposes the slice's public API
  - id: rs
    app: <project-folder>     # Tauri は同一 app 内の言語境界
    path: apps/<project-folder>/src-tauri/src
    language: rust
    layer_set: ddd-vsa-hex-rs
    adapter: rust
    slice_root: <bc>
    slice_subdir: slices
    public_entry: mod.rs
cross_root:                   # optional; 同一 app 内の published-language bridges (Tauri 等)
  - from: { root: rs, path: shared/contracts }
    to:   { root: ts, path: <bc>/types }
    generator: tauri-specta
    auto_generated: true      # the 'to' side is generated; manual edits forbidden
cross_app:                    # optional; monorepo の app 間 contract 同期
  - from: { app: backend,  path: src/<bc>/shared/contracts/events }
    to:   { app: frontend, path: src/<bc>/shared/contracts/events }
    generator: copy-or-publish
    auto_generated: true
layer_sets:
  ddd-vsa-hex-ts: { ... }     # see "Layer set" below
slice_internal:
  slice-internal-ts: { ... }
cross_slice:
  prohibited_direct: true
  via: [shared/contracts, shared/events]
cross_bc:                     # cross-BC bridge(app 内、MVP は単一 event-bus 経由)
  via: [apps/<app>/src/shared/contracts, apps/<app>/src/shared/events]
  same_event_bus: true        # MVP: in-process 単一 bus(app 内)、分散 bus は v0.2+
page_map_marker: phase-11b    # opt-in: enables phase 11b auto-update of UI layer section
phase_hooks:                  # MANDATORY (may be empty `{}`); /ori-arch always emits this block
  flow-impl-red-pre:          # phase name = one of distill-ddd / DoD phases consumed by /ori-flow
    - cmd: cargo run --bin export-types   # bash command to invoke
      cwd: apps/<app>/src-tauri           # optional; cwd for the command (default = repo root)
      reason: "rebuild specta bindings before red boundary tests"  # optional human note
  flow-impl-green-post:
    - cmd: cargo run --bin export-types
      cwd: apps/<app>/src-tauri
      reason: "resync TS bindings after green impl (Slice DoD rule 4)"
---
```

`phase_hooks` lists shell commands `/ori-flow` should run at named phase
transitions. The block is REQUIRED in every architecture.md emitted by
`/ori-arch` (use `phase_hooks: {}` when no hooks are needed, e.g. the plain
`typescript` stack with no `cross_root` contract). The schema is
deliberately loose (`record<string, hook[]>`) so new stack templates can
declare hooks at any phase without a parser change. Consumers (`/ori-flow`,
`/ori-doctor`) treat missing entries as "no-op for that phase".

### Single-root shorthand (v0.1 default)

When the project has exactly one root, omit `roots` and use top-level fields:

```yaml
---
version: 1
workspace:
  apps_root: apps
  apps:
    - name: <project-folder>
      path: apps/<project-folder>
root:                         # singular form — equivalent to roots[0]
  app: <project-folder>
  path: apps/<project-folder>/src
  language: typescript
  layer_set: ddd-vsa-hex-ts
  adapter: eslint
  slice_root: <bc>
  slice_subdir: slices       # optional. when set, slices live at <path>/<bc>/<slice_subdir>/<slice-id>/
  public_entry: index.ts
layer_sets: { ... }
slice_internal: { ... }
cross_slice: { prohibited_direct: true, via: [shared/contracts, shared/events] }
---
```

Adapters MUST treat `root:` and a single-element `roots:` identically.

### `slice_subdir` semantics

`slice_subdir` is optional. When omitted, slices live one directory below
`<root.path>/<slice_root>/<slice-id>/` (the v0.1 layout used by older
templates). When set (typically `slices`), slices descend one more level
to `<root.path>/<slice_root>/<slice_subdir>/<slice-id>/` — this matches
the `apps/<app>/src/<bc>/slices/<slice-id>/` layout declared in
`docs/design.md` §17. BC-internal shared (`kind: shared`) stays at
`<root.path>/<slice_root>/shared/`, i.e. a sibling of `slices/`, not
below it.

---

## Layer set (ddd-vsa-hex)

```yaml
layer_sets:
  ddd-vsa-hex-ts:
    layers:
      - { id: shared,    kind: shared }
      - { id: domain,    kind: slice, slice_internal: slice-internal-ts }
      - { id: ui-widget, kind: ui-layer, order: 1 }
      - { id: ui-page,   kind: ui-layer, order: 2 }
    rules:
      cross_layer:            # one allow-list per source layer; everything else denied
        - { from: ui-page,    allow: [ui-widget, shared, domain] }
        - { from: ui-widget,  allow: [shared, domain] }
        - { from: domain,     allow: [shared] }
        - { from: shared,     allow: [] }
      same_layer: prohibited  # 'prohibited' (default) | 'allowed'
      public_entry_required: true
```

**Layer kinds**:

- `shared` — flat module of cross-cutting code; no internal slice structure
- `slice` — each immediate child is a slice; slice-internal sub-layering applies
- `ui-layer` — UI composition layer. `order` is the topological position; lower = closer to shared

**Default semantics**: same-layer imports are prohibited, public-entry is required.
The schema makes these explicit so that an adapter cannot silently disagree with the
rules. UI-layer rules express a one-way pipeline (`ui-page → ui-widget → shared/domain`),
but a project can deviate by editing `rules.cross_layer` — the schema is policy, not
religion.

**Note on ui-widget**: optional layer for cross-slice UI composition. Projects with
simple UI may omit it and let pages compose slices directly. Phase 11b can declare
widgets via `## Page Map` section (see below).

---

## Slice-internal structure

A slice (member of a `kind: slice` layer) has its own sub-layering corresponding to
ddd-vsa-hex internal layers.

```yaml
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
      - { from: application,    allow: [domain] }
      - { from: infrastructure, allow: [domain] }
      - { from: domain,         allow: [] }
```

The slice-internal sub-layers are **physical directories** under the slice folder
(e.g., `apps/<app>/src/<bc>/slices/register-user/{domain,application,infrastructure,presentation,tests}`).
Adapters resolve them via the layer's `slice_internal` reference.

---

## Cross-slice rule

```yaml
cross_slice:
  prohibited_direct: true   # direct imports between slices are illegal
  via: [shared/contracts, shared/events]   # allowed bridges, relative to root.path/<bc>/
```

`prohibited_direct: true` means an adapter MUST flag any import where the source slice
ID ≠ target slice ID. The only legal cross-slice paths are the ones listed in `via`
(typically `<bc>/shared/contracts/` for domain events and `<bc>/shared/events/` for
the event bus).

---

## Cross-BC rule(MVP, v0.1)

```yaml
cross_bc:
  via: [apps/<app>/src/shared/contracts, apps/<app>/src/shared/events]   # global bridges (app-scoped)
  same_event_bus: true                              # MVP: 単一 event-bus(app 内)、分散 bus は v0.2+
```

BC を跨ぐ場合の bridge は `<root>/shared/contracts/` と `<root>/shared/events/` を経由
(`<root>` は `roots[].path`、apps/ 反映後は `apps/<app>/src` 等)。
v0.1 MVP は同一 app 内の全 BC が同じ in-process event-bus を共有。app をまたぐ場合は
`cross_app:` を declare(monorepo)。v0.2+ で分散 bus、message queue 連携。

---

## Cross-root contracts (multi-root only)

```yaml
cross_root:
  - from: { root: rs, path: shared/contracts }
    to:   { root: ts, path: <bc>/types }
    generator: tauri-specta
    auto_generated: true
```

Declares a **published-language boundary** between two roots. The `to` side is generated
by `generator` from the `from` side. When `auto_generated: true`, adapters on the `to`
root MUST treat the path as read-only and ignore lint errors that originate inside
generated files (they are the source root's problem).

Typical use: Tauri projects where Rust types in `apps/<app>/src-tauri/src/<bc>/shared/contracts/`
become TS types in `apps/<app>/src/<bc>/types/` via `tauri-specta`(同一 app 内の cross-root)。

---

## Markdown body conventions

Below the frontmatter, the body is freeform Markdown for rationale and examples —
humans read it, adapters don't. Two structured sections are recognised:

### `## Layer rationale` (informational)

Free prose explaining why the layer order is what it is, what each layer owns, and any
non-obvious exceptions. Adapters do not read this.

### `## Page Map` (auto-managed by phase 11b)

When `page_map_marker: phase-11b` is set in frontmatter, the
`/ori-ddd-11b-ui-grouping` skill manages the contents of this section between markers:

```markdown
## Page Map

<!-- BEGIN ori-distill phase-11b auto-generated; do not edit between markers -->
- ui-widget:
  - prompt-workspace (depends_on: [prompt-list-slice, prompt-editor-slice])
  - settings-panel (depends_on: [edit-profile-slice, change-password-slice])
- ui-page:
  - registration (depends_on: [register-user, check-username])
  - home (depends_on: [prompt-workspace])
- ui-page:
  - settings (depends_on: [settings-panel])
<!-- END ori-distill phase-11b auto-generated -->

## Manual notes

Anything outside the markers is preserved across regeneration. Use this for opt-outs,
deprecations, or notes the team wants pinned to architecture review.
```

Phase 11b derives this from `.ori/domain/ui-fields/screen-*.md` frontmatter (`depended_by`,
`depends_on`). The markers MUST be byte-exact for regeneration idempotency.

**Note**: `depends_on` references in Page Map are **slice IDs** (or other widgets/pages).
Each slice/page derives_from `page-grouping:<id>` for change-propagation.

---

## Adapter contract

`/ori-arch` exports the spec and invokes the adapter (`./scripts/adapters/<id>.js`, relative to the skill bundle).
Adapters implement:

```ts
// Adapter default export
export interface OriArchAdapter {
  name: string;
  language: string | string[];
  export(spec: ArchitectureSpec, root: RootConfig): Promise<{
    files: { path: string; content: string }[];      // native config files to write
    notes?: string[];                                 // human-facing post-install steps
  }>;
  check?(spec: ArchitectureSpec, root: RootConfig): Promise<{
    violations: { file: string; line?: number; rule: string; message: string }[];
  }>;
}
```

`ArchitectureSpec` is the parsed frontmatter; `RootConfig` is one element of `roots[]`
(or the singular `root` block in shorthand form). Adapters that cannot represent a rule
in their native linter MUST emit a `notes[]` entry rather than silently dropping it.

The MVP adapters in v0.1 scope (all integrated into APM bundle, not separate npm packages):

| Adapter ID | Language(s) | Output                                                |
|------------|-------------|-------------------------------------------------------|
| `eslint`   | TS / JS     | `eslint.config.ori.js` (eslint-plugin-boundaries)     |
| `rust`     | Rust        | `tests/arch.rs` or `cargo-modules` config              |
| `generic`  | any         | `.ori/arch-rules.json` + tiny CLI checker (regex)     |

v0.2+ candidates: `dependency-cruiser` (TS/JS), `import-linter` (Python), `ArchUnit` (JVM).

**Adapter 配置**: skill bundle 内の `./scripts/adapters/<adapter-id>.js` (esbuild bundle
内に統合)。npm package としては配布しない(`@ori-ori/arch-adapter-*` は廃止)。

Contributing new adapter は `docs/contributing/adding-adapter.md` 参照。

---

## Worked example 1 — single-root TypeScript (ships with `ddd-vsa-hex-typescript` template)

```yaml
---
version: 1
workspace:
  apps_root: apps
  apps:
    - name: my-app
      path: apps/my-app
root:
  app: my-app
  path: apps/my-app/src
  language: typescript
  layer_set: ddd-vsa-hex-ts
  adapter: eslint
  slice_root: <bc>
  public_entry: index.ts
layer_sets:
  ddd-vsa-hex-ts:
    layers:
      - { id: shared,    kind: shared }
      - { id: domain,    kind: slice, slice_internal: slice-internal-ts }
      - { id: ui-widget, kind: ui-layer, order: 1 }
      - { id: ui-page,   kind: ui-layer, order: 2 }
    rules:
      cross_layer:
        - { from: ui-page,    allow: [ui-widget, shared, domain] }
        - { from: ui-widget,  allow: [shared, domain] }
        - { from: domain,     allow: [shared] }
        - { from: shared,     allow: [] }
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
cross_slice:
  prohibited_direct: true
  via: [shared/contracts, shared/events]
cross_bc:
  via: [apps/my-app/src/shared/contracts, apps/my-app/src/shared/events]
  same_event_bus: true
page_map_marker: phase-11b
---

## Layer rationale

DDD-VSA-Hex with one-way UI pipeline from pages down to shared. Same-layer imports are
prohibited so widgets can be composed without fearing hidden dependencies. Cross-slice
traffic flows through `<bc>/shared/events` (event-bus) or `<bc>/shared/contracts` (typed
messages) only. Cross-BC bridges live at `apps/<app>/src/shared/`.

## Page Map

<!-- BEGIN ori-distill phase-11b auto-generated; do not edit between markers -->
<!-- (empty until phase 11b runs on .ori/domain/ui-fields/) -->
<!-- END ori-distill phase-11b auto-generated -->
```

## Worked example 2 — multi-root Tauri(同一 app 内の TS/Rust 言語境界)

```yaml
---
version: 1
workspace:
  apps_root: apps
  apps:
    - name: my-tauri-app
      path: apps/my-tauri-app
roots:
  - id: ts
    app: my-tauri-app
    path: apps/my-tauri-app/src
    language: typescript
    layer_set: ddd-vsa-hex-ts
    adapter: eslint
    slice_root: <bc>
    public_entry: index.ts
  - id: rs
    app: my-tauri-app
    path: apps/my-tauri-app/src-tauri/src
    language: rust
    layer_set: ddd-vsa-hex-rs
    adapter: rust
    slice_root: <bc>
    public_entry: mod.rs
cross_root:
  - from: { root: rs, path: shared/contracts }
    to:   { root: ts, path: <bc>/types }
    generator: tauri-specta
    auto_generated: true
layer_sets:
  ddd-vsa-hex-ts: { ... }       # as in example 1
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
  slice-internal-ts: { ... }
  slice-internal-rs:
    sub_layers: [domain, application, infrastructure, presentation]
    rules:
      - { from: presentation,   allow: [application, domain] }
      - { from: application,    allow: [domain] }
      - { from: infrastructure, allow: [domain] }
      - { from: domain,         allow: [] }
cross_slice:
  prohibited_direct: true
  via: [shared/contracts, shared/events]
cross_bc:
  via: [src/shared/contracts, src/shared/events]
  same_event_bus: true
---
```

Each root selects its own adapter; `cross_root` makes the published-language bridge
explicit so adapters know which generated paths to skip.

**Tauri specifics**(同一 app 内の言語境界):
- Rust source: `apps/<app>/src-tauri/src/<bc>/{domain,shared/contracts,slices/<slice>/...,mod.rs}`
- TS generated: `apps/<app>/src/<bc>/types/` (tauri-specta auto-gen)
- TS source (presentation only): `apps/<app>/src/<bc>/slices/<slice>/presentation/`

---

## What's deferred to v2

- **Per-slice overrides** (e.g., a single slice opting into a different `slice_internal`).
  v1 applies the layer's default to every slice.
- **External-package allow-lists** (which npm/crate dependencies each layer may import).
  v1 only governs intra-project boundaries.
- **Glob-based path overrides** for tests, fixtures, examples. v1 treats the
  conventional sub-layer names as fixed.
- **Severity levels** (warn vs error per rule). v1 treats every violation as an error;
  adapters can downgrade in their own config if needed.
- **Distributed cross-BC bridges** (message queue / cross-process event bus). v1 is
  in-process only via `cross_bc.same_event_bus: true`.

Adding any of these is additive: new optional frontmatter fields, default-off, no
migration required.

---

## Related context files

- `.apm/skills/ori-arch/patterns/ddd-vsa-hex/pattern.md` — pattern definition
- `.apm/skills/ori-arch/patterns/ddd-vsa-hex/ai-notes.md` — AI behavior guidance
- `.apm/skills/ori-flow/templates/slice-manifest.yaml.tpl` — `.ori/slices/<id>/manifest.yaml` テンプレート
- `.apm/skills/ori-flow/templates/page-manifest.yaml.tpl` — `.ori/pages/<id>/manifest.yaml` テンプレート
- `docs/contributing/adding-adapter.md` — adapter implementation guide
