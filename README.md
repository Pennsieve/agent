# Agent

A client responsible for running on a host machine and interfacing with the Pennsieve API.

<table>
    <tr>
        <td><strong>Build Status</strong></td>
        <td><img src="https://github.com/Pennsieve/agent/workflows/Testing/badge.svg"></img></td>
    </tr>
</table>

## Current features

| Feature              | Description                                                                         |
| -------------------- | ----------------------------------------------------------------------------------- |
| API Proxy            |                                                                                     |
| CLI                  | A command line interface that allows basic interaction with the Pennsieve platform. |
| File uploading       | Upload or append files to a target package via the command line.                    |
| Timeseries streaming | Stream and cache timeseries data from the Pennsieve platform over a Websocket.      |

## Building

Build artifacts reside in the `${PROJECT_ROOT}/target` directory. The following
commands will work across all major platforms:

- Windows
- macOs
- Linux (Debian)

### Compiling

#### Production

```bash
$ cargo build --release
```

Will output to `${PROJECT_ROOT}/target/release/pennsieve`.

#### Debug

```bash
$ cargo build
```

Will output to `${PROJECT_ROOT}/target/debug/pennsieve`.

This release will contain additional logging output (along with being a lot
faster to compile).

### Generating Documentation

#### End-User

End user documentation is generated from a Jekyll template in `${PROJECT_ROOT}/docs`.

To build the documentation, run

```bash
$ make docs
```

The output will be located in `${PROJECT_ROOT}/docs/_site`.

#### API

API documentation is generated via `cargo`. Running

```bash
$ cargo doc
```

will output to `${PROJECT_ROOT}/target/doc`.

Additionally, you can view the documentation directly in your browser via

```bash
$ cargo doc --open
```

## Testing

To run the Pennsieve agent test suite, run

```bash
$ cargo test
```

### Local Cross OS Testing

Vagrant files for Windows 10 and Debian are included [here](https://github.com/Pennsieve/agent-tools/tree/master/vagrant).

### Run the Agent Against the Development Environment

By default, all connection profiles that are created according to the documentation or using the `config wizard` command will be run against the production environment. In order to run the agent against dev, you must create a profile with your development credentials and then use the `environment` property to specify that this profile is meant to run against dev. Here is a sample profile in `config.ini` that would run against the development environment:

```ini
[new_profile]
api_token=<token>
api_secret=<secret>
environment=development
```

Alternatively, if you are using environment variables to configure the agent, you would set the `PENNSIEVE_API_ENVIRONMENT` environment variable to "development".

## Packaging

Running `make` in `${PROJECT_ROOT}` will create an installation package in the
same directory.

Depending on the operating system, the installer will either be an
`.msi` (Windows), `.pkg` (macOS), or `.deb` (Debian-based Linux distributions).


## Releasing

To build a release for the Pennsieve agent, create an _unannotated_ Git tag and
push it to GitHub.

Tagged release builds are placed in a S3 bucket by Jenkins. They are located
at

```
s3://data.pennsieve.io/public-downloads/agent/${TAG}
```

The latest release is available at

```
s3://data.pennsieve.io/public-downloads/agent/latest
```

## Development

### Generating code from Protobuf definitions

After making the required changes to `resources/proto/timeseries.proto`, you need to run the following commands:

```bash
$ brew install protobuf
$ cargo install protobuf-codegen
$ protoc --rust_out src/ps/proto/ resources/proto/timeseries.proto
```

### Code signing

#### Apple

The Mac installer packages are signed with the Pennsieve Developer ID Installer
certificate.

To verify the signature, run

```
pkgutil --check-signature pennsieve.pkg
```

- [Differences in signing certificates for macOS/iOS](http://jay0lu.github.io/2017/10/17/Apple-Mac-Certificate.html).
- [Managing signing certificates](https://help.apple.com/xcode/mac/current/#/dev154b28f09)
- [Code signing Electron apps](https://www.jviotti.com/2016/03/16/how-to-code-sign-os-x-electron-apps-in-travis-ci.html)
- [Travis CI for iOS](https://www.objc.io/issues/6-build-tools/travis-ci/)
