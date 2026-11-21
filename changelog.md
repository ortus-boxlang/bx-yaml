# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

* * *

## [Unreleased]

## [1.2.0] - 2026-11-21

### Added

- MatchBox backend: the module now also runs on MatchBox without a JVM. A Rust plugin in `native/` provides `yamlDeserialize()`, `yamlDeserializeFile()` and `yamlSerialize()` with the same arguments, packaged as prebuilt libraries for `x86_64` and `aarch64` on Linux, macOS and Windows under `matchbox.artifacts` in the same zip. It reads YAML 1.2 (see the readme for the differences from SnakeYAML) and writes SnakeYAML's block layout. One contract script runs on both backends.

## [1.1.0] - 2026-10-17

### Fixed

- Every YAML sequence deserialized with one extra, spurious array layer (`[ a, b ]` came back as `[ [ a, b ] ]`), because the whole list was passed to the varargs `Array.of()` as a single item. Sequences are now built with `Array.fromList()`. Code that unwrapped the extra layer as a workaround must stop doing so.

## [1.0.0] - 2025-01-28

- First iteration of this module

[unreleased]: https://github.com/ortus-boxlang/bx-yaml/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/ortus-boxlang/bx-yaml/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/ortus-boxlang/bx-yaml/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/ortus-boxlang/bx-yaml/compare/6e3be97d041a46ef67348ce8b5aea00d2c20a496...v1.0.0
