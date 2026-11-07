use std::cmp::Ordering;

/// A parsed version, ordered by Semantic Versioning 2.0.0 precedence: build metadata is ignored, and a release sorts after its own pre-releases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
	major: u64,
	minor: u64,
	patch: u64,
	/// Empty for a release.
	pre_release: Vec<Identifier>,
}

/// Numeric identifiers sort before alphanumeric ones, as Semantic Versioning requires, because of the variant order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Identifier {
	Numeric(u64),
	Alphanumeric(String),
}

impl Version {
	#[must_use]
	pub const fn is_pre_release(&self) -> bool {
		!self.pre_release.is_empty()
	}
}

impl Ord for Version {
	fn cmp(&self, other: &Self) -> Ordering {
		(self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch)).then_with(|| {
			match (self.pre_release.is_empty(), other.pre_release.is_empty()) {
				(true, true) => Ordering::Equal,
				(true, false) => Ordering::Greater,
				(false, true) => Ordering::Less,
				(false, false) => self.pre_release.cmp(&other.pre_release),
			}
		})
	}
}

impl PartialOrd for Version {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

/// Lenient where tags commonly are: a leading `v`, and missing or extra number parts.
pub fn parse_semver(value: &str) -> Option<Version> {
	let normalized = value.trim().trim_start_matches(['v', 'V']);
	// Build metadata goes first, since it may itself contain a `-`.
	let without_build = normalized.split_once('+').map_or(normalized, |(version, _)| version);
	let (numbers, pre_release) = without_build.split_once('-').map_or((without_build, None), |(n, p)| (n, Some(p)));
	let mut parts = numbers.split('.');
	let major = parts.next()?.parse().ok()?;
	let minor = parts.next().unwrap_or("0").parse().ok()?;
	let patch = parts.next().unwrap_or("0").parse().ok()?;
	let pre_release = match pre_release {
		Some(identifiers) => identifiers.split('.').map(parse_identifier).collect::<Option<_>>()?,
		None => Vec::new(),
	};
	Some(Version { major, minor, patch, pre_release })
}

fn parse_identifier(identifier: &str) -> Option<Identifier> {
	if identifier.is_empty() {
		return None;
	}
	if identifier.bytes().all(|b| b.is_ascii_digit()) {
		return identifier.parse().ok().map(Identifier::Numeric);
	}
	Some(Identifier::Alphanumeric(identifier.to_owned()))
}

#[cfg(test)]
mod tests {
	use super::*;

	const fn release(major: u64, minor: u64, patch: u64) -> Version {
		Version { major, minor, patch, pre_release: Vec::new() }
	}

	fn newer(a: &str, b: &str) -> bool {
		parse_semver(a).unwrap() > parse_semver(b).unwrap()
	}

	#[test]
	fn semver_accepts_prefixes_and_prerelease_suffix() {
		assert_eq!(parse_semver("v1.2.3"), Some(release(1, 2, 3)));
		assert_eq!(parse_semver("V4.5.6"), Some(release(4, 5, 6)));
		assert!(parse_semver("1.2.3-beta.1").unwrap().is_pre_release());
	}

	#[test]
	fn semver_defaults_missing_parts() {
		assert_eq!(parse_semver("1"), Some(release(1, 0, 0)));
		assert_eq!(parse_semver("1.2"), Some(release(1, 2, 0)));
	}

	#[test]
	fn semver_rejects_empty_or_invalid() {
		assert_eq!(parse_semver(""), None);
		assert_eq!(parse_semver("not-a-version"), None);
		assert_eq!(parse_semver("v"), None);
		assert_eq!(parse_semver(".2.3"), None);
		assert_eq!(parse_semver("latest"), None);
		assert_eq!(parse_semver("1.0.0-alpha..1"), None);
	}

	#[test]
	fn semver_trims_whitespace() {
		assert_eq!(parse_semver("  v2.3.4  "), Some(release(2, 3, 4)));
	}

	#[test]
	fn semver_ignores_extra_segments() {
		assert_eq!(parse_semver("1.2.3.99"), Some(release(1, 2, 3)));
	}

	/// Someone running a beta must be offered the release it leads up to.
	#[test]
	fn a_release_is_newer_than_its_own_pre_release() {
		assert!(newer("1.1.0", "1.1.0-beta.1"));
		assert!(newer("1.1.0-beta.1", "1.0.9"));
	}

	/// The example ordering in section 11 of Semantic Versioning 2.0.0.
	#[test]
	fn pre_releases_follow_semver_precedence() {
		let ordered = [
			"1.0.0-alpha",
			"1.0.0-alpha.1",
			"1.0.0-alpha.beta",
			"1.0.0-beta",
			"1.0.0-beta.2",
			"1.0.0-beta.11",
			"1.0.0-rc.1",
			"1.0.0",
		];
		for pair in ordered.windows(2) {
			assert!(newer(pair[1], pair[0]), "{} should be newer than {}", pair[1], pair[0]);
		}
	}

	#[test]
	fn build_metadata_is_ignored() {
		assert_eq!(parse_semver("1.0.0+5.g6fd465c"), Some(release(1, 0, 0)));
		assert_eq!(parse_semver("1.0.0-alpha.1+1403.g6fd465c6"), parse_semver("1.0.0-alpha.1"));
		assert_eq!(parse_semver("1.0.0-alpha+build-5"), parse_semver("1.0.0-alpha"));
	}
}
