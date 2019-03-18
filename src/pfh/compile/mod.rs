//! Pertains to compiling a working directory into a library, then executing a function in that library.

mod build;
mod construct;
mod execute;

pub use self::build::{compile, CompilationError};
pub use self::construct::{build_compile_dir, fmt};
pub use self::execute::{exec, exec_and_redirect};

#[cfg(test)]
mod tests {
	use super::*;
	use crate::pfh::*;
	use linking::LinkingConfiguration;
	use std::io::{self, BufRead, BufReader, Write};
	use std::path::{Path, PathBuf};
	use std::process::{Command, Stdio};
	use std::sync::mpsc;
	use std::{error, fmt, fs};

	#[test]
	fn nodata_build_fmt_compile_eval_test() {
		let compile_dir = "test/nodata_build_fmt_compile_eval_test";
		let files = vec![pass_compile_eval_file()];
		let linking_config = LinkingConfiguration::default();

		// build
		build_compile_dir(&compile_dir, files.iter(), &linking_config).unwrap();
		assert!(fs::read_to_string(&format!("{}/src/lib.rs", compile_dir))
			.unwrap()
			.contains("\nlet out0 = 2+2;"));

		// // fmt
		// assert!(fmt(&compile_dir));
		// assert!(fs::read_to_string(&format!("{}/src/lib.rs", compile_dir))
		// 	.unwrap()
		// 	.contains("\n    let out0 = 2 + 2;")); // should be tabbed in (once, unless i wrap it more)

		// compile
		let path = compile(&compile_dir, &linking_config, |_| ()).unwrap();

		// eval
		let r = exec(path, "__intern_eval", ()).unwrap(); // execute library fn

		assert_eq!(&r, "4");
	}

	#[test]
	fn brw_data_build_fmt_compile_eval_test() {
		let compile_dir = "test/brw_data_build_fmt_compile_eval_test";
		let files = vec![pass_compile_eval_file()];
		let linking_config = LinkingConfiguration::default()
			.link_external_crate(
				&compile_dir,
				"papyrus_extern_test",
				Some("test-resources/external_crate/target/debug/libexternal_crate.rlib"),
			)
			.unwrap();

		// build
		build_compile_dir(&compile_dir, files.iter(), &linking_config).unwrap();
		assert!(fs::read_to_string(&format!("{}/src/lib.rs", compile_dir))
			.unwrap()
			.contains("\nlet out0 = 2+2;"));

		// // fmt
		// assert!(fmt(&compile_dir));
		// assert!(fs::read_to_string(&format!("{}/src/lib.rs", compile_dir))
		// 	.unwrap()
		// 	.contains("\n    let out0 = 2 + 2;")); // should be tabbed in (once, unless i wrap it more)

		// compile
		let path = compile(&compile_dir, &linking_config, |_| ()).unwrap();

		// eval
		let r = exec(path, "__intern_eval", &()).unwrap(); // execute library fn

		assert_eq!(&r, "4");
	}

	#[test]
	fn mut_brw_data_build_fmt_compile_eval_test() {
		let compile_dir = "test/mut_brw_data_build_fmt_compile_eval_test";
		let files = vec![pass_compile_eval_file()];
		let linking_config = LinkingConfiguration::default()
			.link_external_crate(
				&compile_dir,
				"papyrus_extern_test",
				Some("test-resources/external_crate/target/debug/libexternal_crate.rlib"),
			)
			.unwrap();

		// build
		build_compile_dir(&compile_dir, files.iter(), &linking_config).unwrap();
		assert!(fs::read_to_string(&format!("{}/src/lib.rs", compile_dir))
			.unwrap()
			.contains("\nlet out0 = 2+2;"));

		// // fmt
		// assert!(fmt(&compile_dir));
		// assert!(fs::read_to_string(&format!("{}/src/lib.rs", compile_dir))
		// 	.unwrap()
		// 	.contains("\n    let out0 = 2 + 2;")); // should be tabbed in (once, unless i wrap it more)

		// compile
		let path = compile(&compile_dir, &linking_config, |_| ()).unwrap();

		// eval
		let r = exec(path, "__intern_eval", ()).unwrap(); // execute library fn

		assert_eq!(&r, "4");
	}

	#[test]
	fn fail_compile_test() {
		let compile_dir = "test/fail_compile";
		let files = vec![faile_compile_file()];
		let linking_config = LinkingConfiguration::default();

		// build
		build_compile_dir(&compile_dir, files.iter(), &linking_config).unwrap();
		assert!(fs::read_to_string(&format!("{}/src/lib.rs", compile_dir))
			.unwrap()
			.contains("\nlet out0 = 2+;"));

		// compile
		let r = compile(&compile_dir, &linking_config, |_| ());
		assert!(r.is_err());
		match r.unwrap_err() {
			CompilationError::CompileError(_) => (),
			_ => panic!("expecting CompileError"),
		}
	}

	#[test]
	fn fail_eval_test() {
		let compile_dir = "test/fail_eval_test";
		let files = vec![fail_eval_file()];
		let linking_config = LinkingConfiguration::default();

		// build
		build_compile_dir(&compile_dir, files.iter(), &linking_config).unwrap();
		assert!(fs::read_to_string(&format!("{}/src/lib.rs", compile_dir))
			.unwrap()
			.contains("\nlet out0 = panic!(\"eval panic\");"));

		// compile
		let path = compile(&compile_dir, &linking_config, |_| ()).unwrap();

		// eval
		let r = exec(&path, "__intern_eval", ()); // execute library fn
		assert!(r.is_err());
		assert_eq!(r, Err("a panic occured with evaluation"));
	}

	fn pass_compile_eval_file() -> SourceFile {
		let mut file = SourceFile::lib();
		file.contents = vec![Input {
			items: vec![],
			stmts: vec![Statement {
				expr: "2+2".to_string(),
				semi: false,
			}],
			crates: vec![],
		}];
		file
	}

	fn faile_compile_file() -> SourceFile {
		let mut file = SourceFile::lib();
		file.contents = vec![Input {
			items: vec![],
			stmts: vec![Statement {
				expr: "2+".to_string(),
				semi: false,
			}],
			crates: vec![],
		}];
		file
	}
	fn fail_eval_file() -> SourceFile {
		let mut file = SourceFile::lib();
		file.contents = vec![Input {
			items: vec![],
			stmts: vec![Statement {
				expr: "panic!(\"eval panic\")".to_string(),
				semi: false,
			}],
			crates: vec![],
		}];
		file
	}
}
