#!/usr/bin/env bash
# ori-generate: generate docker-compose.yml from manifest + architecture.md runtime blocks + infra catalog
# Usage: ./generate-docker-compose.sh <scenario-id>
#
# サービス名解決ルール (design.md §9 / D6):
#   ① workspace.apps と一致 → app service (runtime block [compose-service] から生成)
#   ② infra catalog (scripts/infra-catalog.yaml) と一致 → catalog から生成 (manifest overrides 上書き可)
#   ③ どちらにも不一致 → exit 1 (推測で埋めない)
#
# - local 系 app は compose に含めない (build-then-test。stdout に note を出すだけ)
# - compose-service 系参加者がゼロなら docker-compose.yml を生成しない (既存 file があれば削除)
# - 同一 scenario 内 host port 衝突は exit 1
# - 生成後 docker compose config -q で検証 (docker 不在時は WARN で続行)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null || true)"
if [ -z "$PROJECT_ROOT" ]; then
  d="$SCRIPT_DIR"
  while [ "$d" != "/" ]; do
    if [ -d "$d/.ori" ]; then PROJECT_ROOT="$d"; break; fi
    d="$(dirname "$d")"
  done
fi
if [ -z "$PROJECT_ROOT" ]; then echo "ERROR: cannot find project root (.ori/ not found)" >&2; exit 1; fi
cd "$PROJECT_ROOT"

ID="${1:-}"
if [[ -z "$ID" ]]; then
  echo "ERROR: scenario-id required" >&2
  exit 1
fi

SCENARIO_DIR=".ori/scenarios/$ID"
if [[ ! -f "$SCENARIO_DIR/manifest.yaml" ]]; then
  echo "ERROR: scenario not found: $SCENARIO_DIR/manifest.yaml" >&2
  exit 1
fi

CATALOG="$SCRIPT_DIR/infra-catalog.yaml"
if [[ ! -f "$CATALOG" ]]; then
  echo "ERROR: infra catalog not found: $CATALOG" >&2
  exit 1
fi

ID="$ID" CATALOG="$CATALOG" SCENARIO_DIR="$SCENARIO_DIR" python3 << 'PYEOF'
import yaml
import sys
import os

scenario_id = os.environ['ID']
scenario_dir = os.environ['SCENARIO_DIR']
catalog_path = os.environ['CATALOG']

with open(f"{scenario_dir}/manifest.yaml", "r") as f:
    manifest = yaml.safe_load(f) or {}

infra = manifest.get('infrastructure') or {}
services = infra.get('services') or []
overrides = infra.get('overrides') or {}

if not services:
    print("ERROR: manifest.yaml does not contain infrastructure.services", file=sys.stderr)
    sys.exit(1)

apps = {}
arch_path = ".ori/architecture.md"
if os.path.exists(arch_path):
    with open(arch_path, "r") as f:
        raw = f.read()
    fm = raw.split("---")[1] if raw.startswith("---") else ""
    arch = yaml.safe_load(fm) or {}
    apps = {a['name']: a for a in (arch.get('workspace') or {}).get('apps') or [] if isinstance(a, dict) and 'name' in a}

with open(catalog_path, "r") as f:
    catalog = (yaml.safe_load(f) or {}).get('infra') or {}

compose_services = {}
app_env_hints = {}
local_skipped = []
unresolved = []

for name in services:
    if name in apps:
        runtime = apps[name].get('runtime')
        if not runtime:
            unresolved.append(f"{name}: workspace.apps に存在するが runtime block がない (architecture.md)")
            continue
        if runtime.get('mode') != 'compose-service':
            local_skipped.append(f"{name}: runtime.mode=local (build-then-test。compose には含めない)")
            continue
        parts = [c for c in [runtime.get('install'), runtime.get('build'), runtime.get('run')] if c]
        svc = {
            'image': runtime['image'],
            'working_dir': '/app',
            'volumes': [f"../../../{apps[name]['path']}:/app"],
            'command': ['sh', '-c', ' && '.join(parts)],
        }
        ports = runtime.get('ports') or []
        if ports:
            svc['ports'] = [f"{p}:{p}" for p in ports]
        compose_services[name] = svc
    elif name in catalog:
        entry = catalog[name]
        ov = overrides.get(name) or {}
        svc = {'image': ov.get('image', entry['image'])}
        ports = ov.get('ports', entry.get('ports'))
        if ports:
            svc['ports'] = list(ports)
        env = dict(entry.get('environment') or {})
        env.update(ov.get('environment') or {})
        if env:
            svc['environment'] = env
        if 'healthcheck' in entry:
            svc['healthcheck'] = entry['healthcheck']
        compose_services[name] = svc
        for k, v in (entry.get('app_env') or {}).items():
            app_env_hints[f"{name}:{k}"] = v
    else:
        unresolved.append(f"{name}: workspace.apps にも infra catalog にも不一致 (推測で埋めない)")

if unresolved:
    print("ERROR: サービス名解決不能 — ユーザ確認が必要:", file=sys.stderr)
    for u in unresolved:
        print(f"  - {u}", file=sys.stderr)
    print("  (workspace.apps への runtime block 追加、infra catalog への定義追加、または manifest の名前修正を検討)", file=sys.stderr)
    sys.exit(1)

for note in local_skipped:
    print(f"NOTE: {note}")

host_ports = {}
for name, svc in compose_services.items():
    for p in svc.get('ports', []):
        host = str(p).split(':')[0]
        if host in host_ports:
            print(f"ERROR: host port {host} が {host_ports[host]} と {name} で衝突 (runtime block / catalog の静的宣言を確認)", file=sys.stderr)
            sys.exit(1)
        host_ports[host] = name

out_path = f"{scenario_dir}/docker-compose.yml"
if not compose_services:
    if os.path.exists(out_path):
        os.remove(out_path)
        print(f"Removed (compose-service 系参加者ゼロ): {out_path}")
    else:
        print("compose-service 系参加者ゼロ — docker-compose.yml は省略")
    sys.exit(0)

header = (
    "# @ori-generated scenario:" + scenario_id + "\n"
    "# /ori-generate が生成 (manifest + architecture.md runtime blocks + infra catalog)。\n"
    "# 直接編集には /ori-sync --force が必要。\n"
)
body = yaml.dump({'services': compose_services}, default_flow_style=False, sort_keys=False)
with open(out_path, 'w') as f:
    f.write(header + body)
print(f"Generated: {out_path}")

if app_env_hints:
    print("app env hints (catalog 既定値。app service の environment に追記し、app 固有値は TBD マーカーで残す):")
    for k, v in app_env_hints.items():
        print(f"  {k}={v}")
PYEOF

if [ ! -f "$SCENARIO_DIR/docker-compose.yml" ]; then
  exit 0
fi

if command -v docker >/dev/null 2>&1; then
  if docker compose -f "$SCENARIO_DIR/docker-compose.yml" config -q 2>/dev/null; then
    echo "docker compose config -q: OK"
  else
    echo "ERROR: docker compose config -q failed (self-fix は SKILL.md 手順に従い 1 回まで)" >&2
    exit 1
  fi
else
  echo "WARN: docker 不在のため docker compose config -q 検証を skip" >&2
fi
