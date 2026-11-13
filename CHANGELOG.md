# Changelog

## 0.0.2

- All crates of the workspace (`fs-release`, `fs-release-chain`) share one version, equal to the tag (rule: crate version = tag).
- `workflows/`: five new flows - `frontend-round`, `bugfix`, `review`, `dossier`, `icon-round`; every file has `document.version` 0.0.2.
- Workflow profile: new step field `freesynergy.net:kind` (`objective` = repeat until the goal is reached, `subjective` = change the state whatever the result); required, values checked by `scripts/check_workflows.py`; the five existing flows are marked.

## 0.0.1

- First release.
