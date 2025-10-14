use std::{
	error::Error,
	fmt::{self, Display},
	fs, io,
	path::{Path, PathBuf},
};

use serde::Deserialize;

/// Represents a profile.
#[derive(Debug, Deserialize)]
pub struct Profile {
	/// The display name.
	pub name: String,

	/// The entries to expand for removal.
	pub entries: Vec<Entry>,
}

/// Represents an entry.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Entry {
	/// Represents a path to a single file or directory.
	Path {
		/// The full (or relative) path.
		path: String,
	},

	/// Represents a pattern to match one or more files or directories.
	Pattern {
		/// The full (or relative) pattern to match.
		pattern: String,

		/// The exception to use to ensure one match remains, if any.
		exception: Option<PatternException>,
	},
}

/// Represents a pattern exception.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PatternException {
	/// Indicates the first ascending match.
	FirstAscending,

	/// Indicates the first descending match.
	FirstDescending,

	/// Indicates the most recent (by modified/created timestamp) match.
	MostRecent,
}

/// Represents a profile-related error.
#[derive(Debug)]
pub enum ProfileError {
	/// Indicates that a profile could not be read.
	FailedToRead(io::Error),

	/// Indicates that the JSON representing a profile could not be parsed.
	FailedToDeserialise(serde_json::Error),
}

/// Represents an entry-related error.
#[derive(Debug)]
pub enum EntryError {
	/// Indicates that the glob representing a pattern could not be parsed.
	FailedToParse(glob::PatternError),
}

pub type ProfileResult = Result<Profile, ProfileError>;
pub type EntryResult = Result<Vec<PathBuf>, EntryError>;

impl Profile {
	pub fn load<T>(path: T) -> ProfileResult
	where
		T: AsRef<Path>,
	{
		let json = fs::read_to_string(&path).map_err(ProfileError::FailedToRead)?;
		let profile = serde_json::from_str(&json).map_err(ProfileError::FailedToDeserialise)?;

		Ok(profile)
	}
}

impl Entry {
	/// Expands the entry to the paths it represents.
	/// In the case of a path, this will be a single path.
	/// In the case of a pattern, this will be one or more paths that match the pattern.
	pub fn expand(&self) -> EntryResult {
		match self {
			Self::Path {
				path,
			} => Ok(vec![PathBuf::from(path)]),
			Self::Pattern {
				pattern,
				exception,
			} => {
				let paths: Vec<PathBuf> = match glob::glob(pattern) {
					Ok(p) => p.flatten().collect(),
					Err(e) => return Err(EntryError::FailedToParse(e)),
				};

				let filtered = if let Some(exception) = exception {
					// Determine the path to exclude.

					let exclusion = match &exception {
						PatternException::FirstAscending => paths.iter().min_by_key(|p| p.file_name().map(|n| n.to_str())),
						PatternException::FirstDescending => paths.iter().max_by_key(|p| p.file_name().map(|n| n.to_str())),
						PatternException::MostRecent => paths.iter().max_by_key(|p| p.metadata().and_then(|m| m.modified().or(m.created())).ok()),
					};

					if let Some(e) = exclusion.cloned() {
						paths.into_iter().filter(|p| p != &e).collect()
					} else {
						Vec::default() // Don't expand if no exception
					}
				} else {
					paths
				};

				Ok(filtered)
			}
		}
	}
}

impl Display for Entry {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Entry::Path {
				path,
			} => write!(f, "Path <{}>", path),
			Entry::Pattern {
				pattern,
				exception: _,
			} => write!(f, "Pattern <{}>", pattern),
		}
	}
}

impl Display for PatternException {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::FirstAscending => write!(f, "first-ascending"),
			Self::FirstDescending => write!(f, "first-descending"),
			Self::MostRecent => write!(f, "most-recent"),
		}
	}
}

impl Error for ProfileError {}
impl Error for EntryError {}

impl Display for ProfileError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::FailedToRead(e) => write!(f, "failed to read file [{}]", e),
			Self::FailedToDeserialise(e) => write!(f, "failed to deserialise value [{}]", e),
		}
	}
}

impl Display for EntryError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::FailedToParse(e) => write!(f, "failed to parse glob pattern {}]", e),
		}
	}
}
