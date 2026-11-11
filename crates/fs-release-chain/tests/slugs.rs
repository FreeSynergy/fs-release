//! Test of the strict readers: GitHub URLs, slugs, configured origin, approval path.

mod support;

use fs_release_chain::approval::Approval;
use fs_release_chain::origin::{configured_slug, slug_of};
use fs_release_chain::outcome::Stop;
use support::{git, World};

#[test]
fn only_the_two_exact_github_forms_are_accepted() {
    for good in [
        "https://github.com/FreeSynergy/fs-store",
        "https://github.com/FreeSynergy/fs-store.git",
        "git@github.com:FreeSynergy/fs-store.git",
        "git@github.com:FreeSynergy/fs-store",
    ] {
        assert_eq!(
            slug_of(good).as_deref(),
            Some("FreeSynergy/fs-store"),
            "{good}"
        );
    }
    for bad in [
        "https://evil.example/github.com/FreeSynergy/fs-store",
        "https://github.com.evil.example/FreeSynergy/fs-store",
        "https://user@github.com/FreeSynergy/fs-store",
        "http://github.com/FreeSynergy/fs-store",
        "ssh://git@github.com/FreeSynergy/fs-store",
        "https://github.com/FreeSynergy/fs-store/",
        "https://github.com/FreeSynergy/fs-store/extra",
        "https://github.com/FreeSynergy/f\u{0455}-store",
        "https://github.com/FreeSynergy/fs%2Dstore",
        "https://github.com/FreeSynergy/fs-store\0",
        "https://github.com/FreeSynergy/..",
        "https://github.com/../fs-store",
        "https://github.com/FreeSynergy/",
        "/srv/some/path",
        "",
    ] {
        assert_eq!(slug_of(bad), None, "{bad:?}");
    }
}

#[test]
fn the_configured_origin_must_agree_in_fetch_and_push_url() {
    let w = World::new();
    let runner = w.runner();
    let dir = w.root().join("catalog");
    assert_eq!(configured_slug(&runner, &dir).unwrap(), "Org/fs-catalog");
    git(
        &dir,
        &[
            "config",
            "remote.origin.pushurl",
            "https://github.com/Org/fs-catalog.git",
        ],
    );
    assert!(configured_slug(&runner, &dir).is_ok());
    git(
        &dir,
        &[
            "config",
            "remote.origin.pushurl",
            "https://github.com/Evil/fs-catalog.git",
        ],
    );
    assert!(configured_slug(&runner, &dir)
        .unwrap_err()
        .contains("two repos"));
    git(&dir, &["config", "--unset", "remote.origin.pushurl"]);
    git(
        &dir,
        &[
            "config",
            "remote.origin.url",
            "https://evil.example/github.com/Org/fs-catalog",
        ],
    );
    assert!(configured_slug(&runner, &dir)
        .unwrap_err()
        .contains("not https://github.com"));
    assert!(
        configured_slug(&runner, &w.root()).is_err(),
        "no repo, no origin"
    );
}

#[test]
fn approval_load_refuses_a_slug_or_sha_that_could_leave_the_folder() {
    let w = World::new();
    let dir = w.root().join("freigaben");
    let sha = w.head();
    for slug in ["x/..", "../etc", "x/y/z", ""] {
        assert!(
            matches!(Approval::load(&dir, slug, &sha), Err(Stop::Red { .. })),
            "{slug:?}"
        );
    }
    for head in ["../../x", "abc", &"g".repeat(40), ""] {
        assert!(
            matches!(
                Approval::load(&dir, "Org/fs-demo", head),
                Err(Stop::Red { .. })
            ),
            "{head:?}"
        );
    }
}
