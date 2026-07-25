# CI
Standards: `https://git.iot/Infra/Docs/raw/branch/main/01-network/ci-cd/standards.md` · Kiosk: `…/kiosk-api-guide.md`
## Workflows
| file | trigger | jobs |
|-|-|-|
| `ci.yaml` | push main · PR · dispatch | validate (tree-gate · hardcode-gate · docs-gate) |
| `mirror-github.yaml` | push main · dispatch | mirror → GitHub (deploy key, `.gitea` stripped) |
| `ship.yaml` | tag `ship/esp-s3-hal-*` · dispatch | validate · kiosk-health · kiosk-e2e |
## Gates
- **tree-gate**: required dirs/files exist
- **hardcode-gate**: no hostnames/IPs in workflow YAML (except `ci.yaml`)
- **docs-gate**: every `docs/*.md` + `README.md` + `AGENTS.md` ≤ 100 lines
## Secrets / Vars
| name | kind | use |
|-|-|-|
| `GH_DEPLOY_KEY` | secret | base64 ed25519 private key for GitHub push |
| `ESP_S3_HAL_GH_REPO` | var | GitHub repo path |
| `ESP_S3_HAL_GH_BRANCH` | var | GitHub branch |
| `GH_SSH_HOST` | var | GitHub SSH host |
| `RUNNER_T1` / `RUNNER_T4` | var | runner labels |
| `KIOSK_FQDN` / `KIOSK_API_PORT` | var | kiosk endpoints |
Zero hardcode in YAML — all hosts via vars/secrets.
