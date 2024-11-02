use std::{
	error::Error,
	fmt::{self, Display},
	fs, io, iter,
	path::{Path, PathBuf},
};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Profile {
	pub name: String,
	pub entries: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum Entry {
	File(String),
	Dir(String),
	Pattern(String),
}

#[derive(Debug)]
pub enum ProfileError {
	FailedToRead(io::Error),
	FailedToDeserialise(serde_json::Error),
}

#[derive(Debug)]
pub enum EntryError {
	FailedToParse(glob::PatternError),
}

pub type ProfileResult = Result<Profile, ProfileError>;
pub type EntryResult = Result<Box<dyn Iterator<Item = PathBuf>>, EntryError>;

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
	pub fn expand(&self) -> EntryResult {
		match self {
			Self::File(f) => Ok(Box::new(iter::once(PathBuf::from(f)))),
			Self::Dir(d) => Ok(Box::new(iter::once(PathBuf::from(d)))),
			Self::Pattern(p) => match glob::glob(p) {
				Ok(p) => Ok(Box::new(p.flatten())),
				Err(e) => Err(EntryError::FailedToParse(e)),
			},
		}
	}
}

impl Display for Entry {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::File(i) => write!(f, "File <{}>", i),
			Self::Dir(d) => write!(f, "Dir <{}>", d),
			Self::Pattern(p) => write!(f, "Pattern ({})", p),
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
