//! The command-line reader the guards share, from
//! [`writfold_core::command`]: simple commands with quotes and escapes
//! removed, here-document bodies set aside as data, and each command's
//! invocation and directory. It never runs or expands anything.
pub use writfold_core::command::{
    Dir, Invocation, Why, Word, command, invocations, matches, segments,
};
