//! # papyrus
//! 
//! [![Build Status](https://travis-ci.com/kurtlawrence/papyrus.svg?branch=master)](https://travis-ci.com/kurtlawrence/papyrus) [![Latest Version](https://img.shields.io/crates/v/papyrus.svg)](https://crates.io/crates/papyrus) [![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://docs.rs/papyrus) [![codecov](https://codecov.io/gh/kurtlawrence/papyrus/branch/master/graph/badge.svg)](https://codecov.io/gh/kurtlawrence/papyrus)
//! 
//! ## A rust REPL and script running tool
//! 
//! See the [rs docs](https://docs.rs/papyrus/) and the [usage guide](https://kurtlawrence.github.io/papyrus/)
//! Look at progress and contribute on [github.](https://github.com/kurtlawrence/papyrus)
//! 
//! ## Installation
//! 
//! `papyrus` depends on `proc-macro2` and `syn` which contains features that are only available on a nightly compiler. Further to this, the features are underneath a config flag, so compiling requires the `RUSTFLAGS` environment variable to include `--cfg procmacro2_semver_exempt`.
//! 
//! Switch to a nightly compiler.
//! 
//! ```sh
//! rustup default nightly
//! ```
//! 
//! Linux, Mac
//! 
//! ```bash
//! RUSTFLAGS="--cfg procmacro2_semver_exempt" cargo install papyrus
//! ```
//! 
//! Windows
//! 
//! ```bash
//! $env:RUSTFLAGS="--cfg procmacro2_semver_exempt"
//! cargo install papyrus
//! ```
//! 
//! ## REPL
//! 
//! `papyrus run` will start the repl!
//! 
//! ## Shell Context Menu
//! 
//! Add right click context menu. (May need admin rights)
//! 
//! ```bash
//! papyrus rc-add
//! ```
//! 
//! Remove right click context menu. (May need admin rights)
//! 
//! ```bash
//! papyrus rc-remove
//! ```
//! 
//! Run papyrus from command line.
//! 
//! ```bash
//! papyrus run path_to_src_file.rs
//! papyrus run path_to_script_file.rscript
//! ```
//! 
//! ## Implementation Notes
//! 
//! - Right click on a `.rs` or `.rscript` file and choose `Run with Papyrus` to compile and run code!
//! - Papyrus will take the contents of the source code and construct a directory to be used with `cargo`. For now the directory is created under a `.papyrus` directory in the users home directory.
//! - The compiled binary will be executed with the current directory the one that houses the file. So `env::current_dir()` will return the directory of the `.rs` or `.rscript` file.
//! 
//! ## Example - .rs
//! 
//! File `hello.rs`.
//! 
//! ```sh
//! extern crate some_crate;
//! 
//! fn main() {
//!   println!("Hello, world!");
//! }
//! ```
//! 
//! Use papyrus to execute code.
//! 
//! ```bash
//! papyrus run hello.rs
//! ```
//! 
//! The `src/main.rs` will be populated with the same contents as `hello.rs`. A `Cargo.toml` file will be created, where `some_crate` will be added as a dependency `some-crate = "*"`.
//! 
//! ## Example - .rscript
//! 
//! File `hello.rscript`.
//! 
//! ```sh
//! extern crate some_crate;
//! 
//! println!("Hello, world!");
//! ```
//! 
//! Use papyrus to execute code.
//! 
//! ```bash
//! papyrus run hello.rscript
//! ```
//! 
//! The `src/main.rs` will be populated with a main function encapsulating the code, and crate references placed above it. A similar `Cargo.toml` will be created as before.
#![feature(test)]

#[macro_use]
extern crate log;

#[cfg(test)]
extern crate test;

extern crate colored;
extern crate dirs;
extern crate failure;
extern crate linefeed;
extern crate proc_macro2;
extern crate syn;
extern crate term_cursor;
extern crate term_size;

mod compile;
mod contextmenu;
mod file;
mod input;
mod repl;

use failure::ResultExt;
use std::{fs, path};

pub use self::compile::Exe;
pub use self::contextmenu::{add_right_click_menu, remove_right_click_menu};
pub use self::file::{CrateType, SourceFile, SourceFileType};
pub use self::repl::Repl;
pub use self::repl::{CmdArgs, Command};

const PAPYRUS_SPLIT_PATTERN: &'static str = "<!papyrus-split>";
#[cfg(test)]
const RS_FILES: [&'static str; 2] = ["with-crate.rs", "pwr.rs"];
#[cfg(test)]
const RSCRIPT_FILES: [&'static str; 7] = [
	"expr.rscript",
	"one.rscript",
	"expr-list.rscript",
	"count_files.rscript",
	"items.rscript",
	"dir.rscript",
	"use_rand.rscript",
];

/// Creates the specified file along with the directory to it if it doesn't exist.
fn create_file_and_dir<P: AsRef<path::Path>>(
	file: &P,
) -> Result<fs::File, failure::Context<String>> {
	let file = file.as_ref();
	match file.parent() {
		Some(parent) => {
			fs::create_dir_all(parent).context(format!("failed creating directory {:?}", parent))?
		}
		None => (),
	}

	fs::File::create(file).context(format!("failed creating file {:?}", file))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn create_file_and_dir_test() {
		let p = path::Path::new("foo.txt");
		assert!(!p.exists());
		create_file_and_dir(&"foo.txt").unwrap();
		assert!(p.exists());
		fs::remove_file(p).unwrap();
		assert!(!p.exists());

		let p = path::Path::new("tests/foo");
		assert!(!p.exists());
		create_file_and_dir(&p).unwrap();
		assert!(p.exists());
		fs::remove_file(p).unwrap();
		assert!(!p.exists());
	}
}
