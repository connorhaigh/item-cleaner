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
		/// The path.
		path: PathBuf,
	},

	/// Represents a pattern to match one or more files or directories.
	Pattern {
		/// The pattern to match.
		pattern: String,

		/// The retention to use, if any.
		retention: Option<Retention>,
	},
}

/// Represents a retention.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Retention {
	/// The (ascending) order to use for sorting matches.
	pub order: Order,

	/// The number of matches to retain.
	pub count: usize,
}

/// Represents an order.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Order {
	/// Indicates to be sorted by file name.
	FileName,

	/// Indicates to be sorted by the first created timestamp.
	Created,

	/// Indicates to be sorted by the last modified timestamp.
	Modified,
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
	pub fn expand(self) -> EntryResult {
		match self {
			Self::Path {
				path,
			} => Ok(vec![path]),
			Self::Pattern {
				pattern,
				retention,
			} => {
				// Expand the initial set of paths from the pattern.

				let mut paths: Vec<PathBuf> = match glob::glob(&pattern) {
					Ok(p) => p.flatten().collect(),
					Err(e) => return Err(EntryError::FailedToParse(e)),
				};

				// Sort and omit the paths that should be retained, if any.

				if let Some(retention) = retention {
					paths.sort_by(|a, b| match &retention.order {
						Order::FileName => a.file_name().cmp(&b.file_name()),
						Order::Created => b.metadata().and_then(|m| m.created()).ok().cmp(&a.metadata().and_then(|m| m.created()).ok()),
						Order::Modified => b.metadata().and_then(|m| m.modified()).ok().cmp(&a.metadata().and_then(|m| m.modified()).ok()),
					});

					paths.drain(0..retention.count);
				}

				Ok(paths)
			}
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
			Self::FailedToParse(e) => write!(f, "failed to parse glob pattern [{}]", e),
		}
	}
}
