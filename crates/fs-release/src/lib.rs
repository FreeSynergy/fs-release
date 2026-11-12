//! Pattern: none (module list of `fs-release`).
//!
//! `fs-release lift <repo> --components vX --interfaces vY` is one deterministic command (no LLM): lift the dependencies,
//! regenerate the lock, run the lock gate and the checks, write a report. `flow` goes on: review-request, tag, push.
//! Every unit of work is a [`task::Task`]; a later Serverless-Workflow file only has to name the same tasks.

pub mod cli;
pub mod lift;
pub mod task;
