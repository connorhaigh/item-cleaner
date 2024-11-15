use std::{
	error::Error,
	fmt::{self, Display},
	fs,
	io::{self, BufRead, Write},
	path::{Path, PathBuf},
	time::Instant,
};

use clap::{Parser, ValueEnum};
use humansize::{FormatSize, DECIMAL};
use profile::{Entry, Profile, ProfileError};

mod profile;

/// Performs cleaning on directories using profiles.
#[derive(Debug, Parser)]
#[command(author, version, about, long_about)]
struct Args {
	/// Specifies the profile file
	#[arg(short, long)]
	profile: PathBuf,

	/// Specifies the clean mode
	#[arg(short, long, value_enum, default_value_t=Mode::EveryPath)]
	mode: Mode,
}

/// Determines the mode of operation.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Mode {
	/// Indicates that no prompts should be generated.
	Silent,

	/// Indicates that every entry should be prompted for confirmation before removing.
	EveryEntry,

	/// Indicates that every expanded path should be prompted for confirmation before removing.
	EveryPath,
}

fn main() {
	let args = Args::parse();

	match clean(args.profile, args.mode) {
		Ok(()) => println!("Successfully cleaned items."),
		Err(e) => println!("Failed to clean items: {}.", e),
	}
}

/// Represents a clean-related error.
#[derive(Debug)]
enum CleanError {
	/// Indicates that the profile could not be loaded.
	FailedToLoad(ProfileError),

	/// Indicates that the entry could not be removed.
	FailedToRemove(RemoveError),
}

/// Represents a remove-related error.
#[derive(Debug)]
enum RemoveError {
	/// Indicates that the metadata for a particular file could not be read.
	FailedToInspectEntry(io::Error),

	/// Indicates that a particular file could not be removed.
	FailedToRemoveFile(io::Error),

	/// Indicates that a particular directory could not be removed.
	FailedToRemoveDirectory(io::Error),

	/// Indicates that a particular directory could not be read for its files.
	FailedToReadDirectory(io::Error),
}

/// Indicates the result of a clean operation.
type CleanResult = Result<(), CleanError>;

/// Indicates the result of a remove operation.
type RemoveResult = Result<u64, RemoveError>;

impl Display for CleanError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::FailedToLoad(e) => write!(f, "failed to load profile [{}]", e),
			Self::FailedToRemove(e) => write!(f, "failed to remove entry [{}]", e),
		}
	}
}

impl Display for RemoveError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::FailedToInspectEntry(e) => write!(f, "failed to inspect entry [{}]", e),
			Self::FailedToRemoveFile(e) => write!(f, "failed to remove file [{}]", e),
			Self::FailedToRemoveDirectory(e) => write!(f, "failed to remove directory [{}]", e),
			Self::FailedToReadDirectory(e) => write!(f, "failed to read directory files [{}]", e),
		}
	}
}

impl Error for CleanError {}
impl Error for RemoveError {}

/// Cleans the entries described by the specified profile in the specified mode.
fn clean<T>(profile: T, mode: Mode) -> CleanResult
where
	T: AsRef<Path>,
{
	println!("Loading profile from path <{}>...", profile.as_ref().display());

	let profile = Profile::load(profile).map_err(CleanError::FailedToLoad)?;

	println!("Discovering paths using profile '{}'...", profile.name);

	// Collect all applicable entries.

	#[rustfmt::skip]
	let entries: Vec<&Entry> = if matches!(mode, Mode::EveryEntry) {
		profile.entries.iter()
			.filter(|&entry| prompt(format!("Include entry [{}]?", entry)))
			.collect()
	} else {
		profile.entries.iter().collect()
	};

	let start = Instant::now();

	// Expand each entry to all of its paths.

	#[rustfmt::skip]
	let paths: Vec<PathBuf> = entries.iter()
		.flat_map(|&e| e.expand())
		.flatten()
		.flat_map(|p| p.canonicalize())
		.collect();

	println!("Expanded {} paths in {:#?}.", paths.len(), start.elapsed());
	println!("Deleting {} paths...", paths.len());

	let start = Instant::now();

	let mut total = 0usize;
	let mut size = 0u64;

	for (index, path) in paths.iter().enumerate() {
		if matches!(mode, Mode::EveryPath) {
			if !prompt(format!("Delete path <{}>?", path.display())) {
				continue;
			}
		} else {
			println!("Deleting path {} of {}: <{}>...", index + 1, paths.len(), path.display());
		}

		match remove(path).map_err(CleanError::FailedToRemove) {
			Ok(s) => {
				total += 1;
				size += s;
			}
			Err(e) => {
				println!("Failed to delete path: {}.", e);
			}
		}
	}

	println!("Deleted {} paths in {:#?}, reclaiming {} of space.", total, start.elapsed(), size.format_size(DECIMAL));

	Ok(())
}

/// Attempts to remove the specified path.
fn remove<T>(path: T) -> RemoveResult
where
	T: AsRef<Path>,
{
	let metadata = path.as_ref().metadata().map_err(RemoveError::FailedToInspectEntry)?;

	match &metadata {
		m if m.is_file() => {
			fs::remove_file(path).map_err(RemoveError::FailedToRemoveFile)?;

			Ok(metadata.len())
		}
		m if m.is_dir() => {
			#[rustfmt::skip]
			let size = fs::read_dir(&path).map_err(RemoveError::FailedToReadDirectory)?
				.flatten().map(|e| remove(e.path()))
				.flatten().sum();

			fs::remove_dir(path).map_err(RemoveError::FailedToRemoveDirectory)?;

			Ok(size)
		}
		_ => Ok(0u64),
	}
}

/// Continually prompts for a yes or no answer.
fn prompt<T>(str: T) -> bool
where
	T: AsRef<str>,
{
	loop {
		print!("{} (Y/n): ", str.as_ref());

		let _ = io::stdout().flush();
		let Some(Ok(line)) = io::stdin().lock().lines().next() else {
			return false;
		};

		return match line.as_str() {
			"Y" | "y" => true,
			"N" | "n" => false,
			_ if line.is_empty() => true,
			_ => continue,
		};
	}
}
