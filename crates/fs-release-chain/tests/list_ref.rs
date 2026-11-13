//! Test of the ref the pre-approved list comes from: always `origin/main` of fs-release, never the run's `--base`.

mod support;

use fs_release_chain::meta::Reach;
use fs_release_chain::Outcome;
use support::flow::{all_green, stopped_at};
use support::{git, World};

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(ToString::to_string).collect()
}

fn run_with_base(w: &World, base: &str) -> Outcome {
    let mut t = w.target();
    t.base = base.into();
    w.run_target(t, Reach::Public, &names(&["review"]))
}

#[test]
fn another_base_branch_cannot_bring_its_own_list() {
    let w = World::new();
    let sha = w.head();
    w.approve(&sha, "JA");
    all_green(&w, &sha);
    w.set_preapproved(&["Org/fs-demo"], "");
    // The same list with `public_ok` filled, published on another branch of the fs-release origin only.
    w.write_preapproved(&["Org/fs-demo"], &["Org/fs-catalog"], "forged", false);
    git(
        &w.root().join("rel"),
        &["push", "--quiet", "origin", "HEAD:refs/heads/release-x"],
    );
    let o = run_with_base(&w, "release-x");
    assert_eq!(
        stopped_at(&o),
        "halt preflight public-locked",
        "the list of another base counts for nothing: {}",
        o.line()
    );
    // Control: the same file on main opens the lock, whatever the base is.
    git(
        &w.root().join("rel"),
        &["push", "--quiet", "origin", "HEAD:refs/heads/main"],
    );
    let o = run_with_base(&w, "release-x");
    assert_ne!(
        stopped_at(&o),
        "halt preflight public-locked",
        "{}",
        o.line()
    );
}

#[test]
fn a_base_that_looks_like_a_git_option_is_red_before_git_sees_it() {
    let w = World::new();
    let sha = w.head();
    w.approve(&sha, "JA");
    all_green(&w, &sha);
    w.set_preapproved(&["Org/fs-demo"], "ok");
    for base in ["-x", "--upload-pack=touch pwned", "a b", "x..y", ""] {
        let o = run_with_base(&w, base);
        assert_eq!(stopped_at(&o), "red preflight", "{base:?}: {}", o.line());
    }
}
