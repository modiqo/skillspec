#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: skillspec-clean-slate [--yes] [--dry-run] [--no-backup]

Remove only SkillSpec-managed local state for a fresh install test.

Targets:
  ~/.skillspec
  ~/.agents/skills/{skillspec,skillspec-creator,durable-executor,skill-router}
  ~/.agent/skills/{skillspec,skillspec-creator,durable-executor,skill-router}
  ~/.codex/skills/{skillspec,skillspec-creator,durable-executor,skill-router}
  ~/.claude/skills/{skillspec,skillspec-creator,durable-executor,skill-router}

By default, this creates a backup under ~/skillspec-clean-slate-backup-<stamp>
and asks for confirmation. Use --yes for non-interactive deletion.
USAGE
}

confirm=false
dry_run=false
backup=true

while [[ $# -gt 0 ]]; do
  case "$1" in
    --yes|-y)
      confirm=true
      ;;
    --dry-run|-n)
      dry_run=true
      ;;
    --no-backup)
      backup=false
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

skill_names=(skillspec skillspec-creator durable-executor skill-router)
skill_roots=(
  "$HOME/.agents/skills"
  "$HOME/.agent/skills"
  "$HOME/.codex/skills"
  "$HOME/.claude/skills"
)

targets=()
if [[ -e "$HOME/.skillspec" || -L "$HOME/.skillspec" ]]; then
  targets+=("$HOME/.skillspec")
fi

for root in "${skill_roots[@]}"; do
  [[ -d "$root" || -L "$root" ]] || continue
  for name in "${skill_names[@]}"; do
    path="$root/$name"
    if [[ -e "$path" || -L "$path" ]]; then
      targets+=("$path")
    fi
  done
done

if [[ ${#targets[@]} -eq 0 ]]; then
  echo "skillspec clean slate: no SkillSpec-managed targets found"
  exit 0
fi

echo "skillspec clean slate targets:"
for target in "${targets[@]}"; do
  echo "  $target"
done

if [[ "$dry_run" == true ]]; then
  echo "dry run: no files changed"
  exit 0
fi

backup_dir=""
if [[ "$backup" == true ]]; then
  stamp="$(date +%Y%m%d-%H%M%S)"
  backup_dir="$HOME/skillspec-clean-slate-backup-$stamp"
  mkdir -p "$backup_dir"
  for target in "${targets[@]}"; do
    relative="${target#$HOME/}"
    parent="$backup_dir/$(dirname "$relative")"
    mkdir -p "$parent"
    cp -a "$target" "$parent/"
  done
  echo "backup: $backup_dir"
fi

if [[ "$confirm" != true ]]; then
  printf "Type DELETE to remove these SkillSpec-managed targets: "
  read -r answer
  if [[ "$answer" != "DELETE" ]]; then
    echo "aborted"
    exit 1
  fi
fi

if command -v skillspec >/dev/null 2>&1; then
  skillspec durable-executor delete --json >/dev/null 2>&1 || true
  skillspec router uninstall --json >/dev/null 2>&1 || true
fi

for target in "${targets[@]}"; do
  rm -rf "$target"
done

echo "skillspec clean slate complete"
if [[ -n "$backup_dir" ]]; then
  echo "backup: $backup_dir"
fi
