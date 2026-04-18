# v0.3.3 - 2026-04-18

## Fixed
- Fix syntax error in statistics.rs (invalid comparison operator)
- Fix duplicate `json_file` in arg.rs conflicts_with_all
- Move test module in dispatcher.rs to end of file (clippy::items_after_test_module)
- Replace `assert_eq!(.., true/false)` with `assert!(..)` for boolean assertions (clippy::bool_assert_comparison)

## Updated
- Update dependencies to latest versions (tokio, zerocopy, wasm-bindgen, etc.)
- Improve code style and consistency

# v0.3.2 - 2026-03-03

## Fixed
- Fix GitHub Actions Release workflow variable reference issues
- Add semantic version validation for release tags
- Improve error handling and logging in CI/CD workflows

## Added
- Enhance test coverage from 57% to 80%+ with 93 new test cases
- Add comprehensive tests for arg, output, and statistics modules
- Add tests for parse_percentiles, parse_filename_and_path functions
- Add tests for HTTP method conversions and output format handling
- Add tests for JSON serialization/deserialization
- Add tests for statistics calculations and message handling
- Add boundary condition tests for HTTP status codes

# v0.3.1 - 2025-03-03

Upgrade Rust edition to 2024, update dependencies, and fix CI issues.

# v0.2.2 - 2024-01-24

Fix: docker images is broken.

# v0.2.1 - 2023-10-30

Update Dependencies.

# v0.2.0 - 2023-07-30

build json request body from external command.

# v0.1.12 - 2023-07-30

update dependency and slim docker images.

# v0.1.11 - 2023-07-30

static link everything on unix platform.

# v0.1.10 - 2023-07-30

static link everything on unix platform.

# v0.1.9 - 2023-06-26

Update Dependencies.

# v0.1.8 - 2023-06-17

Update Dependencies.

# v0.1.7 - 2023-04-18

:wave::wave::wave:, Fix too many open files in unix system. 

# v0.1.6 - 2023-04-02

add docker image to ghcr.io.

# v0.1.5 - 2023-03-19

compared with the previous version, no code changes have been made, but a corresponding code scanning tool has been added to the warehouse.

# v0.1.2 - 2023-03-19

:wave::wave::wave:

`rsb` v0.1.0 is the first version, the main function of this tool is to realize the pressure test on the Http server. The development of this tool is mainly inspired by the [`bombardier`](https://github.com/codesenberg/bombardier) project, and I would like to thank the author for his contribution. On the other hand, this tool was developed primarily to learn and understand Rust.

# v0.1.1 - 2023-03-18

:wave::wave::wave:

`rsb` v0.1.0 is the first version, the main function of this tool is to realize the pressure test on the Http server. The development of this tool is mainly inspired by the [`bombardier`](https://github.com/codesenberg/bombardier) project, and I would like to thank the author for his contribution. On the other hand, this tool was developed primarily to learn and understand Rust.

![rsb-basic](resources/assets/basic.gif)

# v0.1.0 - 2023-03-18

:wave::wave::wave::thumbsup::thumbsup::thumbsup:

`rsb` v0.1.0 is the first version, the main function of this tool is to realize the pressure test on the Http server. The development of this tool is mainly inspired by the [`bombardier`](https://github.com/codesenberg/bombardier) project, and I would like to thank the author for his contribution. On the other hand, this tool was developed primarily to learn and understand Rust.

