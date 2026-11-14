# Red notice: `_notify-red.yml` (event `net.freesynergy.run.failed`)

Silence on green. A red run opens ONE issue in its own repo, `rot: <workflow> <repo>`, label `rot`; a second red run of the same
workflow only comments; the next green run closes it. No token beyond `GITHUB_TOKEN`, nothing across repos. The coordinator asks
once for the whole organisation: `gh search issues --owner FreeSynergy --label rot --state open` (empty = nothing to do).

## Use (template for the last job of a workflow of a repo)

```yaml
  notify-red:
    needs: [check]            # every job whose red matters
    if: always()
    permissions:
      issues: write
    uses: FreeSynergy/fs-release/.github/workflows/_notify-red.yml@main
    with:
      workflow: check
      failed: ${{ contains(needs.*.result, 'failure') }}
```

The calling job must grant `issues: write` itself: a called workflow can never have more rights than its caller.

## Why it is not wired into `check.yml` and `release.yml` here

Both are called by many program repos, whose calling jobs grant only `contents: read` (check) or the release rights. A job
inside the reusable workflow that asks for `issues: write` would exceed what those callers grant and make EVERY run of those
repos fail at start. So the wiring is a step per repo (the template above, together with `issues: write` in the caller), done
when the repos are lifted, not a silent change here. Until then a repo without the job simply raises no issue.

## Limits

- A run that cannot start at all (schedule switched off after 60 days without activity, start failure) raises no issue.
- `gh issue list --search` is fuzzy, so the job selects the exact title itself.
