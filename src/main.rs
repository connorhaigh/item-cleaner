use std::{
	error::Error,
	fmt::{self, Display},
	fs,
	io::{self},
	path::{Path, PathBuf},
	time::Instant,
};

use clap::Parser;
use humansize::{FormatSize, DECIMAL};
use profile::{Profile, ProfileError};

mod profile;

/// Performs cleaning on directories using profiles.
#[derive(Debug, Parser)]
#[command(author, version, about, long_about)]
struct Args {
	/// Specifies the profile file
	#[arg(short, long)]
	profile: PathBuf,
}

fn main() {
	let args = Args::parse();

	match clean(args.profile) {
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
	FailedToInspectPath(io::Error),

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
			Self::FailedToRemove(e) => write!(f, "failed to remove path [{}]", e),
		}
	}
}

impl Display for RemoveError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::FailedToInspectPath(e) => write!(f, "failed to inspect path [{}]", e),
			Self::FailedToRemoveFile(e) => write!(f, "failed to remove file [{}]", e),
			Self::FailedToRemoveDirectory(e) => write!(f, "failed to remove directory [{}]", e),
			Self::FailedToReadDirectory(e) => write!(f, "failed to read directory files [{}]", e),
		}
	}
}

impl Error for CleanError {}
impl Error for RemoveError {}

/// Cleans the entries described by the specified profile in the specified mode.
fn clean<T>(profile: T) -> CleanResult
where
	T: AsRef<Path>,
{
	println!("Loading profile from path <{}>...", profile.as_ref().display());

	let profile = Profile::load(profile).map_err(CleanError::FailedToLoad)?;

	println!("Expanding paths using profile '{}'...", profile.name);

	let start = Instant::now();

	// Expand each entry to all of its paths.

	#[rustfmt::skip]
	let paths: Vec<PathBuf> = profile.entries.into_iter()
		.flat_map(|e| e.expand()).flatten()
		.flat_map(|p| p.canonicalize())
		.collect();

	println!("Expanded {} paths in {:#?}.", paths.len(), start.elapsed());
	println!("Deleting {} paths...", paths.len());

	let start = Instant::now();

	// Iterate through each path and remove it.

	let mut total = 0usize;
	let mut size = 0u64;

	for (index, path) in paths.iter().enumerate() {
		println!("Deleting path {} of {}: <{}>...", index + 1, paths.len(), path.display());

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
	let metadata = path.as_ref().metadata().map_err(RemoveError::FailedToInspectPath)?;

	match &metadata {
		m if m.is_dir() => {
			#[rustfmt::skip]
			let size = fs::read_dir(&path).map_err(RemoveError::FailedToReadDirectory)?
				.flatten().map(|e| remove(e.path()))
				.flatten().sum();

			fs::remove_dir(path).map_err(RemoveError::FailedToRemoveDirectory)?;

			Ok(size)
		}
		m if m.is_file() => {
			fs::remove_file(path).map_err(RemoveError::FailedToRemoveFile)?;

			Ok(metadata.len())
		}
		_ => Ok(0u64),
	}
}
