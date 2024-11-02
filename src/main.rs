use std::{
	error::Error,
	fmt::{self, Display},
	fs,
	io::{self, BufRead, Write},
	path::{Path, PathBuf},
	time::Instant,
};

use clap::{Parser, ValueEnum};
use profile::{Entry, Profile, ProfileError};

mod profile;

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

	match clean(&args.profile, args.mode.into()) {
		Ok(()) => println!("Successfully cleaned items."),
		Err(e) => println!("Failed to clean items: {}.", e),
	}
}

#[derive(Debug)]
enum CleanError {
	FailedToLoad(ProfileError),
	FailedToInspectEntry(io::Error),
	FailedToRemoveFile(io::Error),
	FailedToRemoveDirectory(io::Error),
}

type CleanResult = Result<(), CleanError>;

impl Display for CleanError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::FailedToLoad(e) => write!(f, "failed to load profile [{}]", e),
			Self::FailedToInspectEntry(e) => write!(f, "failed to inspect entry [{}]", e),
			Self::FailedToRemoveDirectory(e) => write!(f, "failed to remove directory [{}]", e),
			Self::FailedToRemoveFile(e) => write!(f, "failed to remove file [{}]", e),
		}
	}
}

impl Error for CleanError {}

fn clean<T>(profile_path: T, mode: Mode) -> CleanResult
where
	T: AsRef<Path>,
{
	println!("Loading profile from path <{}>...", profile_path.as_ref().display());

	let profile = Profile::load(profile_path).map_err(CleanError::FailedToLoad)?;

	println!("Discovering paths using profile '{}'...", profile.name);

	#[rustfmt::skip]
	let entries: Vec<&Entry> = if matches!(mode, Mode::EveryEntry) {
		profile.entries.iter()
			.filter(|&entry| prompt(format!("Include entry [{}]?", entry)))
			.collect()
	} else {
		profile.entries.iter().collect()
	};

	let start = Instant::now();

	#[rustfmt::skip]
	let paths: Vec<PathBuf> = entries.iter()
		.flat_map(|&e| e.expand())
		.flatten()
		.flat_map(|p| p.canonicalize())
		.collect();

	println!("Expanded {} paths in {:#?}.", paths.len(), start.elapsed());
	println!("Deleting paths...");

	for (index, path) in paths.iter().enumerate() {
		if matches!(mode, Mode::EveryPath) {
			if !prompt(format!("Delete path <{}>?", path.display())) {
				continue;
			}
		} else {
			println!("Deleting path {} of {}: <{}>...", index + 1, paths.len(), path.display());
		}

		if let Err(e) = remove(path) {
			println!("Failed to delete path: {}.", e);
		}
	}

	Ok(())
}

fn remove<T>(path: T) -> CleanResult
where
	T: AsRef<Path>,
{
	match path.as_ref().metadata().map_err(CleanError::FailedToInspectEntry)? {
		m if m.is_file() => fs::remove_file(&path).map_err(CleanError::FailedToRemoveFile),
		m if m.is_dir() => fs::remove_dir_all(&path).map_err(CleanError::FailedToRemoveDirectory),
		_ => Ok(()),
	}
}

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
