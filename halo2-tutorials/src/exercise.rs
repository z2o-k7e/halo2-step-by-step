use regex::Regex;
use serde::Deserialize;
// use std::env;
use std::fmt::{self, Display, Formatter};
use std::fs::{remove_file, File};
use std::io::Read;
use std::path::PathBuf;
use std::process::{self, Command};

const RUSTC_COLOR_ARGS: &[&str] = &["--color", "always"];
const RUSTC_EDITION_ARGS: &[&str] = &["--edition", "2021"];
const RUSTC_NO_DEBUG_ARGS: &[&str] = &["-C", "strip=debuginfo"];
const I_AM_DONE_REGEX: &str = r"(?m)^\s*///?\s*I\s+AM\s+NOT\s+DONE";
const CONTEXT: usize = 2;
// const CLIPPY_CARGO_TOML_PATH: &str = "./exercises/22_clippy/Cargo.toml";

// Get a temporary file name that is hopefully unique
#[inline]
fn temp_file() -> String {
    let thread_id: String = format!("{:?}", std::thread::current().id())
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect();
    // println!("temp_file format {:?}", format!("./temp_{}_{thread_id}", process::id()));
    format!("./temp_{}_{thread_id}", process::id())
}

// The mode of the exercise.
#[derive(Deserialize, Copy, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    // Indicates that the exercise should be compiled as a binary
    Compile,
    // Indicates that the exercise should be compiled as a test harness
    Test,
}

#[derive(Deserialize)]
pub struct ExerciseList {
    pub exercises: Vec<Exercise>,
}

// A representation of a rustlings exercise.
// This is deserialized from the accompanying info.toml file
#[derive(Deserialize, Debug)]
pub struct Exercise {
    // Name of the exercise
    pub name: String,
    // The path to the file containing the exercise's source code
    pub test_mod: String,
    pub path: PathBuf,
    pub feature: String,
    // The mode of the exercise (Test, Compile, or Clippy)
    pub mode: Mode,
    // The hint text associated with the exercise
    pub hint: String,
}

// An enum to track of the state of an Exercise.
// An Exercise can be either Done or Pending
#[derive(PartialEq, Debug)]
pub enum State {
    // The state of the exercise once it's been completed
    Done,
    // The state of the exercise while it's not completed yet
    Pending(Vec<ContextLine>),
}

// The context information of a pending exercise
#[derive(PartialEq, Debug)]
pub struct ContextLine {
    // The source code that is still pending completion
    pub line: String,
    // The line number of the source code still pending completion
    pub number: usize,
    // Whether or not this is important
    pub important: bool,
}

// The result of compiling an exercise
pub struct CompiledExercise<'a> {
    exercise: &'a Exercise,
    _handle: FileHandle,
}

impl<'a> CompiledExercise<'a> {
    // Run the compiled exercise
    pub fn run(&self) -> Result<ExerciseOutput, ExerciseOutput> {
        self.exercise.run()
    }
}

// A representation of an already executed binary
#[derive(Debug)]
pub struct ExerciseOutput {
    // The textual contents of the standard output of the binary
    pub stdout: String,
    // The textual contents of the standard error of the binary
    pub stderr: String,
}

struct FileHandle;

impl Drop for FileHandle {
    fn drop(&mut self) {
        clean();
    }
}

impl Exercise {
    pub fn compile(&self) -> Result<CompiledExercise, ExerciseOutput> {
        let cmd = match self.mode {
            Mode::Compile => {
                // println!("Mode::Compile {:?}", Mode::Compile);
                Command::new("rustc")
                    .args([self.path.to_str().unwrap(), "-o", &temp_file()])
                    .args(RUSTC_COLOR_ARGS)
                    .args(RUSTC_EDITION_ARGS)
                    .args(RUSTC_NO_DEBUG_ARGS)
                    .output()
            },

            Mode::Test => {
                println!("self.test_mod.as_str() {:?}", self.test_mod.as_str());
                Command::new("cargo")
                // .args(&["test", "--", "--nocapture", "chap_1::exercise_1::tests::test_chap_1"])
                .args(&["test", 
                        "--features", self.feature.as_str(), 
                        "--", "--nocapture", self.test_mod.as_str()])
                .output()
            },
            // Mode::Test => {
            //   // println!("Mode::Test {:?}", Mode::Test);
            //   Command::new("cargo")
            //     .args(["test","--", "--nocapture", "chap_1::exercise_2::tests::test_chap_1"])
            //     // .args(["--test", self.path.to_str().unwrap(), "-o", &temp_file()])
            //     // .args(RUSTC_COLOR_ARGS)
            //     // .args(RUSTC_EDITION_ARGS)
            //     // .args(RUSTC_NO_DEBUG_ARGS)
            //     .output()
            //   },
            // Mode::Test => Command::new("rustc")
            //     .args(["--test", self.path.to_str().unwrap(), "-o", &temp_file()])
            //     .args(RUSTC_COLOR_ARGS)
            //     .args(RUSTC_EDITION_ARGS)
            //     .args(RUSTC_NO_DEBUG_ARGS)
            //     .output(),
        }.expect("Failed to run 'compile' command.");
        // println!("cmd.status {:?}", cmd.status);
        if cmd.status.success() {
            Ok(CompiledExercise {
                exercise: self,
                _handle: FileHandle,
            })
        } else {
            clean();
            Err(ExerciseOutput {
                stdout: String::from_utf8_lossy(&cmd.stdout).to_string(),
                stderr: String::from_utf8_lossy(&cmd.stderr).to_string(),
            })
        }
    }

    fn run(&self) -> Result<ExerciseOutput, ExerciseOutput> {
        let output = ExerciseOutput{
          stdout: String::from(""),
          stderr: String::from(""),
        };
        return Ok(output);
        // let arg = match self.mode {
        //     Mode::Test => "--show-output",
        //     _ => "",
        // };
        // println!("run arg .. {:?}", arg);
        // let cmd = Command::new(temp_file())
        //     .arg(arg)
        //     .output()
        //     .expect("Failed to run 'run' command");

        // let output = ExerciseOutput {
        //     stdout: String::from_utf8_lossy(&cmd.stdout).to_string(),
        //     stderr: String::from_utf8_lossy(&cmd.stderr).to_string(),
        // };

        // if cmd.status.success() {
        //     Ok(output)
        // } else {
        //     Err(output)
        // }
    }

    pub fn state(&self) -> State {
        let mut source_file = File::open(&self.path).unwrap_or_else(|e| {
            panic!(
                "We were unable to open the exercise file {}! {e}",
                self.path.display()
            )
        });

        let source = {
            let mut s = String::new();
            source_file.read_to_string(&mut s).unwrap_or_else(|e| {
                panic!(
                    "We were unable to read the exercise file {}! {e}",
                    self.path.display()
                )
            });
            s
        };

        let re = Regex::new(I_AM_DONE_REGEX).unwrap();

        if !re.is_match(&source) {
            return State::Done;
        }

        let matched_line_index = source
            .lines()
            .enumerate()
            .find_map(|(i, line)| if re.is_match(line) { Some(i) } else { None })
            .expect("This should not happen at all");

        let min_line = ((matched_line_index as i32) - (CONTEXT as i32)).max(0) as usize;
        let max_line = matched_line_index + CONTEXT;

        let context = source
            .lines()
            .enumerate()
            .filter(|&(i, _)| i >= min_line && i <= max_line)
            .map(|(i, line)| ContextLine {
                line: line.to_string(),
                number: i + 1,
                important: i == matched_line_index,
            })
            .collect();

        State::Pending(context)
    }

    // Check that the exercise looks to be solved using self.state()
    // This is not the best way to check since
    // the user can just remove the "I AM NOT DONE" string from the file
    // without actually having solved anything.
    // The only other way to truly check this would to compile and run
    // the exercise; which would be both costly and counterintuitive
    pub fn looks_done(&self) -> bool {
        self.state() == State::Done
    }
}

impl Display for Exercise {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.path.to_str().unwrap())
    }
}

#[inline]
fn clean() {
    let _ignored = remove_file(temp_file());
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_clean() {
        File::create(temp_file()).unwrap();
        let exercise = Exercise {
            name: String::from("example"),
            path: PathBuf::from("tests/fixture/state/pending_exercise.rs"),
            mode: Mode::Compile,
            hint: String::from(""),
            test_mod: String::from(""),
            feature: String::from(""),
        };
        let compiled = exercise.compile().unwrap();
        drop(compiled);
        assert!(!Path::new(&temp_file()).exists());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_no_pdb_file() {
        [Mode::Compile, Mode::Test] // Clippy doesn't like to test
            .iter()
            .for_each(|mode| {
                let exercise = Exercise {
                    name: String::from("example"),
                    // We want a file that does actually compile
                    path: PathBuf::from("tests/fixture/state/pending_exercise.rs"),
                    mode: *mode,
                    hint: String::from(""),
                };
                let _ = exercise.compile().unwrap();
                assert!(!Path::new(&format!("{}.pdb", temp_file())).exists());
            });
    }

    #[test]
    fn test_pending_state() {
        let exercise = Exercise {
            name: "pending_exercise".into(),
            path: PathBuf::from("tests/fixture/state/pending_exercise.rs"),
            mode: Mode::Compile,
            hint: String::new(),
            test_mod: String::new(),
            feature: String::from(""),
        };

        let state = exercise.state();
        let expected = vec![
            ContextLine {
                line: "// fake_exercise".to_string(),
                number: 1,
                important: false,
            },
            ContextLine {
                line: "".to_string(),
                number: 2,
                important: false,
            },
            ContextLine {
                line: "// I AM NOT DONE".to_string(),
                number: 3,
                important: true,
            },
            ContextLine {
                line: "".to_string(),
                number: 4,
                important: false,
            },
            ContextLine {
                line: "fn main() {".to_string(),
                number: 5,
                important: false,
            },
        ];

        assert_eq!(state, State::Pending(expected));
    }

    #[test]
    fn test_finished_exercise() {
        let exercise = Exercise {
            name: "finished_exercise".into(),
            path: PathBuf::from("tests/fixture/state/finished_exercise.rs"),
            mode: Mode::Compile,
            hint: String::new(),
            test_mod: String::new(),
            feature: String::from(""),
        };

        assert_eq!(exercise.state(), State::Done);
    }

    #[test]
    fn test_exercise_with_output() {
        let exercise = Exercise {
            name: "exercise_with_output".into(),
            path: PathBuf::from("tests/fixture/success/testSuccess.rs"),
            mode: Mode::Test,
            hint: String::new(),
            test_mod: String::new(),
            feature: String::from(""),
        };
        // println!("test_exercise_with_outpu  exercise  {:?}", exercise);
        let out = exercise.compile().unwrap(); //.run().unwrap();
        // assert!(out.stdout.contains("THIS TEST TOO SHALL PASS"));
    }
}
