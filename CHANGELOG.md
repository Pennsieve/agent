# Changelog

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)

This project is under active development. Strict versioning and backwards compatibility will not be guaranteed until version 1.0.0.

Documentation at https://developer.pennsieve.io/agent

## [0.2.6]
### Added
- Indications in the progress bar for when uploads fail

## [0.2.5]
### Fixed
- Upload preview was being truncated incorrectly for lots of packages vs packages with lots of files

## [0.2.4]
### Fixed
- Progress bar in `ManyFiles` mode was not properly updating completed file uploads for packages with lots of files
- Updated to pennsieve-rust version that will properly bubble up authorization errors so we can retry them

## [0.2.3]
### Added
- A new `upload-verify` command that will compare an upload hash according to the upload service with a hash that the agent calculates based on the local file in order to verify upload integrity

## [0.2.2]
### Added
- Bump pennsieve-rust version to 0.12.0

## [0.2.1]
### Fixed
- Notifications for new Agent versions are written to stderr instead of stdout

## [0.2.0]
### Added
- Support for `PENNSIEVE_LOG_LEVEL`, `PENNSIEVE_API_TOKEN`, and `PENNSIEVE_API_SECRET` environment variables
- Websocket server for queuing uploads and providing status updates
- `move` (alias `mv`) command for moving packages and collections

### Changed
- The Windows executable is now named `pennsieve.exe`
- Dataset names can now be passed to commands in addition to dataset ids
- All errors now return non-zero exit codes
- When more than 30 files are uploaded, progress updates are rendered in a more compact manner

### Fixed
- Uploads from network drives on Windows computers
- Excessive memory consumption when uploading large numbers of files
- Upload errors caused by tokens expiring during long uploads
- Bugs in the rendering of progress bars

### Removed
- The `--upload_service` flag from the `upload` and `append` commands. The agent now uses this service by default. The deprecated direct-to-S3 upload can still be specified with the `--legacy` flag.
