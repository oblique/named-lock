# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [unreleased]

### Added

- Added `NamedLock::with_path` on UNIX (#2, #4)

### Changed

- `NamedLock::create` on UNIX respects `TMPDIR` enviroment variable (#1, #4)
- `NamedLock::create` now rejects names that contain `/` or `\` characters (#2, #4)
- `NamedLock::create` on Windows explicitly creates a global mutex
- `Error::CreateFailed` now has the source of the error
- Upgrade all dependencies
