# Changelog

## 0.0.3

- All crates and all flows at 0.0.3 (crate version = tag).
- `workflows/`: 13 new flows - `workflow-pflege`, `coordination-round`, `cost-reconcile`, `report-to-strategy`, `slot-check`, `wip-continue`, `download-readonly`, `publish-doc` (coordination) and `adr-write`, `picture-acceptance`, `user-decision`, `handoff-update`, `restart-handoff` (strategy); actor type and kind per step.
- `release-flow.yaml`: the real steps between push and release were missing - `ci` (CI of release-<version> green) and `main-ff` (fast-forward main, human or coordinator).
- `workflows/README.md`: rule "every work step runs through a workflow", list of the flows, how an order names a flow. The schema is unchanged; the test now also checks that the file list equals the folder and that every flow carries the crate version.

- Fix after review: the coordinator is modelled the same everywhere - push, tag and merge steps are `actor: human`, `gate: human` (`publish-doc` push, `adr-write` push were `ai`); `gate: coordinator` is gone (now `human`). Pure computing and reading steps are `machine` (`cost-reconcile` compare/unknown/correction/limit, `cost-log`, slots `log`, `acceptance` of `frontend-round`); `ai` only on a deviation or red (new steps `explain` in `cost-reconcile`, `acceptance-red` in `frontend-round`, both with `if`).
- All `@fs` calls, `chains/release.chain.yaml` (`version`, `catalog`) and the doc comment in `chain.rs` are at 0.0.3 (were mixed with 0.1.0); a test checks it. `DESIGN.md`: flows list and kind corrected, the false "no CHANGELOG" note removed.
- `frontend-round`: the `dossier` step carries the human gate of its sub-workflow's publish; the user's verdict on the images (`picture-verdict`, picture-acceptance) comes after `images`; the second review is gone. `bugfix`: no extra review - the one Opus review of a head is `review` in `release-flow` (`icon-round` has its own single review and no release-flow).

## 0.0.2

- All crates of the workspace (`fs-release`, `fs-release-chain`) share one version, equal to the tag (rule: crate version = tag).
- `workflows/`: five new flows - `frontend-round`, `bugfix`, `review`, `dossier`, `icon-round`; every file has `document.version` 0.0.2.
- Workflow profile: new step field `freesynergy.net:kind` (`objective` = repeat until the goal is reached, `subjective` = change the state whatever the result); required, values checked by `scripts/check_workflows.py`; the five existing flows are marked.

## 0.0.1

- First release.
