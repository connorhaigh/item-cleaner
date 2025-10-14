# item-cleaner

`item-cleaner` is a Rust-based command-line application that can be used to prune many temporary files based on different profiles.

## Overview

The general idea of this application is that it can be used to remove a configurable collection of paths dictated by individual profiles; that is, to say, a list of entries that may either represent a directory, a file, or a glob pattern.

This is primarily to facilitate the automatic removal of many various temporary directories and files that can be created (and left) by a variety of different applications, with the intent to reclaim space that is otherwise in use by them as the application itself may not perform any sort of clean-up properly.

## Usage

Clean using the profile at the specified path, asking for confirmation on every path to remove:

```
item-cleaner --profile safe.json --mode every-path
```

Clean using the profile at the specified path, without any confirmation:

```
item-cleaner --profile nuclear.json --mode silent
```

## Profiles

Profiles are represented by individual JSON files, which contain the name of the profile as well as the entries to be removed whenever the profile is used. An entry can either be a path to a directory, a path to a file, or a [glob-like pattern](https://en.wikipedia.org/wiki/Glob_(programming)) which is subsequently expanded.

For a relatively example, to create a profile named 'Simple' that deletes a single file, it would appear as follows:

```json
{
	"name": "Simple",
	"entries": [
		{ "type": "file", "value": "C:\\hello.txt" }
	]
}
```

For a more complex example, to create a profile named 'Complex' that deletes a single file, a single directory, and anything that matches a glob pattern (except the most recent), it would appear as follows:

```json
{
	"name": "Complex",
	"entries": [
		{ "type": "file", "value": "C:\\hello.txt" },
		{ "type": "directory", "value": "C:\\logs" },
		{ "type": "pattern", "value": "C:\\dumps\\*\\*.dmp", "exception": "mostRecent" }
	]
}
```
