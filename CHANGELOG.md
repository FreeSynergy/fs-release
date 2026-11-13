# Changelog

## 0.0.3

- All crates and all flows at 0.0.3 (crate version = tag).
- `workflows/`: 13 new flows - `workflow-pflege`, `coordination-round`, `cost-reconcile`, `report-to-strategy`, `slot-check`, `wip-continue`, `download-readonly`, `publish-doc` (coordination) and `adr-write`, `picture-acceptance`, `user-decision`, `handoff-update`, `restart-handoff` (strategy); actor type and kind per step.
- `release-flow.yaml`: the real steps between push and release were missing - `ci` (CI of release-<version> green) and `main-ff` (fast-forward main, human or coordinator).
- `workflows/README.md`: rule "every work step runs through a workflow", list of the flows, how an order names a flow. The schema is unchanged; the test now also checks that the file list equals the folder and that every flow carries the crate version.

## 0.0.2

- All crates of the workspace (`fs-release`, `fs-release-chain`) share one version, equal to the tag (rule: crate version = tag).
- `workflows/`: five new flows - `frontend-round`, `bugfix`, `review`, `dossier`, `icon-round`; every file has `document.version` 0.0.2.
- Workflow profile: new step field `freesynergy.net:kind` (`objective` = repeat until the goal is reached, `subjective` = change the state whatever the result); required, values checked by `scripts/check_workflows.py`; the five existing flows are marked.

## 0.0.1

- First release.
