//! Test of the ref the pre-approved list comes from: always `origin/main` of fs-release, never the run's `--base`.

mod support;

use fs_release_chain::meta::Reach;
use fs_release_chain::Outcome;
use std::fs;

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

/// A forged local `refs/remotes/origin/main` (`public_ok` filled) and an origin that cannot be fetched.
fn forged_local_ref_and_dead_origin(w: &World) {
    let sha = w.head();
    w.approve(&sha, "JA");
    all_green(w, &sha);
    w.set_preapproved(&["Org/fs-demo"], "");
    w.write_preapproved(&["Org/fs-demo"], &["Org/fs-catalog"], "forged", false);
    let rel = w.root().join("rel");
    git(&rel, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    fs::remove_dir_all(w.root().join("rel-origin.git")).unwrap();
}

#[test]
fn public_reach_never_falls_back_to_a_local_list_when_the_fetch_fails() {
    let w = World::new();
    forged_local_ref_and_dead_origin(&w);
    let o = run_with_base(&w, "main");
    assert_eq!(stopped_at(&o), "red preflight", "{}", o.line());
    assert!(o.line().contains("no local ref is trusted"), "{}", o.line());
}

#[test]
fn below_public_reach_the_existing_ref_is_still_used_when_the_fetch_fails() {
    let w = World::new();
    forged_local_ref_and_dead_origin(&w);
    let mut t = w.target();
    t.base = "main".into();
    let o = w.run_target(t, Reach::Shared, &names(&["review"]));
    assert_ne!(stopped_at(&o), "red preflight", "{}", o.line());
}

#[test]
fn bump_wants_the_kind_of_release_the_run_asked_for() {
    let w = World::new();
    let sha = w.head();
    w.approve(&sha, "JA");
    all_green(&w, &sha);
    w.set_preapproved(&["Org/fs-demo"], "ok");
    // Published, but as a full release while the run asked for a pre-release.
    fs::write(
        w.fake().join("release-Org_fs-demo-v0.0.9.json"),
        "{\"isDraft\":false,\"isPrerelease\":false}",
    )
    .unwrap();
    let o = w.run_target(w.target(), Reach::Public, &names(&["bump"]));
    assert_eq!(stopped_at(&o), "red bump", "{}", o.line());
}
