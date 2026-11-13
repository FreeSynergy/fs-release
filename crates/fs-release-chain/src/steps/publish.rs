//! Pattern: Strategy, part of the chain commands (the three commands that move refs on the remote: push, merge-ff, tag).

use crate::git::{self, short};
use crate::meta::{Meta, Reach, Risk};
use crate::outcome::{Note, StepResult, Stop};
use crate::steps::ci::verify_green;
use crate::steps::{Frame, Step};

/// Pushes the branch (never forced: a rejected push is red).
pub struct Push;

impl Step for Push {
    fn meta(&self) -> Meta {
        Meta {
            name: "push",
            risk: Risk::Publish,
            reach: Reach::Public,
            preview: "{{branch}} is pushed to {{remote}}.",
            undo: "none",
        }
    }

    fn run(&self, f: &Frame) -> StepResult {
        let t = &f.ctx.target;
        if git::remote_branch(f.ctx, &t.repo_dir, &t.branch)?.as_deref() == Some(f.head) {
            return Ok(Note::text(format!(
                "{} is at {} on the remote already",
                t.branch,
                short(f.head)
            )));
        }
        let spec = format!("{}:refs/heads/{}", f.head, t.branch);
        let o = git::raw(f.ctx, &t.repo_dir, &["push", "origin", &spec])?;
        if o.ok() {
            Ok(Note::text(format!(
                "pushed {} at {}",
                t.branch,
                short(f.head)
            )))
        } else {
            Err(Stop::red(format!("push rejected: {}", o.first_error())))
        }
    }
}

/// Moves the base branch forward to exactly the checked commit, without a merge commit. Asks GitHub for the CI result
/// itself; the run log is not believed.
pub struct MergeFf;

impl Step for MergeFf {
    fn meta(&self) -> Meta {
        Meta {
            name: "merge-ff",
            risk: Risk::Publish,
            reach: Reach::Public,
            preview: "{{base}} is moved forward to {{sha}} without a merge commit.",
            undo: "none",
        }
    }

    fn run(&self, f: &Frame) -> StepResult {
        let t = &f.ctx.target;
        verify_green(f)?;
        let Some(base) = git::remote_branch(f.ctx, &t.repo_dir, &t.base)? else {
            return Err(Stop::red(format!(
                "{} does not exist on the remote",
                t.base
            )));
        };
        if base == f.head {
            return Ok(Note::text(format!(
                "{} is at {} already",
                t.base,
                short(f.head)
            )));
        }
        git::fetch(f.ctx, &t.repo_dir, &t.base)?;
        if !git::is_ancestor(f.ctx, &t.repo_dir, &base, f.head)? {
            return Err(Stop::red(format!(
                "{} moved to {}: rebase needed (new CI, new approval)",
                t.base,
                short(&base)
            )));
        }
        let spec = format!("{}:refs/heads/{}", f.head, t.base);
        let o = git::raw(f.ctx, &t.repo_dir, &["push", "origin", &spec])?;
        if o.ok() {
            Ok(Note::text(format!("{} moved to {}", t.base, short(f.head))))
        } else {
            Err(Stop::red(format!("merge-ff rejected: {}", o.first_error())))
        }
    }
}

/// Sets the release tag on the base branch head and pushes it. A tag is never moved.
pub struct Tag;

impl Step for Tag {
    fn meta(&self) -> Meta {
        Meta {
            name: "tag",
            risk: Risk::Publish,
            reach: Reach::Public,
            preview: "The tag {{tag}} is set on {{sha}} and pushed.",
            undo: "none",
        }
    }

    fn run(&self, f: &Frame) -> StepResult {
        let t = &f.ctx.target;
        let tag = t.tag();
        let name = format!("tags/{tag}");
        if let Some(at) = git::remote_ref(f.ctx, &t.repo_dir, &name)? {
            return if at == f.head {
                Ok(Note::text(format!("{tag} is on {} already", short(f.head))))
            } else {
                Err(Stop::red(format!(
                    "tag {tag} exists on {}, not on {}",
                    short(&at),
                    short(f.head)
                )))
            };
        }
        if git::remote_branch(f.ctx, &t.repo_dir, &t.base)?.as_deref() != Some(f.head) {
            return Err(Stop::red(format!(
                "{} is not at {}: run merge-ff first",
                t.base,
                short(f.head)
            )));
        }
        match git::rev(f.ctx, &t.repo_dir, &format!("refs/tags/{tag}^{{commit}}")) {
            Ok(local) if local != f.head => {
                return Err(Stop::red(format!(
                    "local tag {tag} is on {}, not on {}",
                    short(&local),
                    short(f.head)
                )));
            }
            Ok(_) => {}
            Err(_) => {
                git::out(f.ctx, &t.repo_dir, &["tag", "-a", &tag, f.head, "-m", &tag])?;
            }
        }
        let o = git::raw(
            f.ctx,
            &t.repo_dir,
            &["push", "origin", &format!("refs/tags/{tag}")],
        )?;
        if o.ok() {
            Ok(Note::text(format!("tag {tag} pushed on {}", short(f.head))))
        } else {
            Err(Stop::red(format!("tag push rejected: {}", o.first_error())))
        }
    }
}
