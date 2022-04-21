# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [unreleased]

## [0.2.0]

### Added

- Added `NamedLock::with_path` on UNIX ([#2], [#4])

### Changed

- `NamedLock::create` on UNIX respects `TMPDIR` environment variable ([#1], [#4])
- `NamedLock::create` now rejects names that contain `/` or `\` characters ([#2], [#4])
- `NamedLock::create` on Windows explicitly creates a global mutex
- `Error::CreateFailed` now has the source of the error
- Upgrade all dependencies


[unreleased]: https://github.com/oblique/named-lock/compare/0.2.0...HEAD
[0.2.0]: https://github.com/oblique/named-lock/compare/0.1.1...0.2.0

[#4]: https://github.com/oblique/named-lock/issues/4
[#2]: https://github.com/oblique/named-lock/issues/2
[#1]: https://github.com/oblique/named-lock/issues/1
