# Contributing to EdgeFirst Recorder

Thank you for your interest in contributing to EdgeFirst Recorder! This document provides guidelines for contributing to this project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Code Style Guidelines](#code-style-guidelines)
- [Testing Requirements](#testing-requirements)
- [Pull Request Process](#pull-request-process)
- [Developer Certificate of Origin (DCO)](#developer-certificate-of-origin-dco)
- [License](#license)

## Code of Conduct

This project adheres to the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to support@au-zone.com.

## Getting Started

EdgeFirst Recorder is an MCAP data recording node for the EdgeFirst Perception stack, bridging Zenoh topics to self-describing MCAP files for offline analysis. Before contributing:

1. Read the [README.md](README.md) to understand the project
2. Browse the [EdgeFirst Documentation](https://doc.edgefirst.ai/latest/maivin/) for context
3. Check existing [issues](https://github.com/EdgeFirstAI/recorder/issues) and [discussions](https://github.com/EdgeFirstAI/recorder/discussions)
4. Review the [EdgeFirst Samples](https://github.com/EdgeFirstAI/samples) to see usage examples

## Development Setup

### Prerequisites

- **Rust**: 1.75 or later (pinned via `rust-version` in `Cargo.toml`; [install instructions](https://rustup.rs/))
- **Git**: For version control

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/EdgeFirstAI/recorder.git
cd recorder

# Build
cargo build
```

### Rust Development

```bash
cargo fmt         # Format code
cargo clippy      # Run linter
cargo test        # Run tests
cargo doc         # Generate documentation
```

## How to Contribute

### Reporting Bugs

Before creating bug reports, please check existing issues to avoid duplicates.

**Good Bug Reports** include:

- Clear, descriptive title
- Steps to reproduce the behavior
- Expected vs. actual behavior
- Environment details (OS, Rust version)
- Minimal code example demonstrating the issue

### Contributing Code

1. **Fork the repository** and create your branch from `main`
2. **Make your changes** following our code style guidelines
3. **Add tests** for new functionality (minimum 70% coverage)
4. **Ensure all tests pass** (`cargo test`)
5. **Update documentation** for API changes
6. **Run formatters and linters** (`cargo fmt`, `cargo clippy`)
7. **Submit a pull request** with a clear description

## Code Style Guidelines

### Rust Guidelines

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` (enforced in CI)
- Address all `cargo clippy` warnings
- Write doc comments for public APIs
- Maximum line length: 100 characters

## Testing Requirements

All contributions with new functionality should include tests. Coverage is
measured but not gated in CI today; target ~70 % overall with full coverage
on critical paths (topic normalisation, timestamp extraction, schema
registry).

### Running Tests

```bash
cargo test              # Run the unit test suite
make test               # Runs cargo-nextest under cargo-llvm-cov and
                        # writes target/rust-coverage.lcov
```

See [TESTING.md](TESTING.md) for the Foxglove-based manual verification
workflow that complements unit tests.

## Pull Request Process

### Branch Naming

```text
feature/<description>       # New features
bugfix/<description>        # Bug fixes
docs/<description>          # Documentation updates
```

### Commit Messages

Write clear, concise commit messages:

```text
Add [feature] for [purpose]

- Implementation detail 1
- Implementation detail 2
```

**Guidelines:**

- Use imperative mood ("Add feature" not "Added feature")
- First line: 50 characters or less
- Body: Wrap at 72 characters

### Pull Request Checklist

Before submitting, ensure:

- [ ] Code follows style guidelines (`cargo fmt`, `cargo clippy`)
- [ ] All tests pass (`cargo test`)
- [ ] New tests added for new functionality
- [ ] Documentation updated for API changes
- [ ] SPDX headers present in new files

## Developer Certificate of Origin (DCO)

All contributors must sign off their commits:

```bash
git commit -s -m "Add new feature"
```

**Configure git:**

```bash
git config user.name "Your Name"
git config user.email "your.email@example.com"
```

## License

By contributing to EdgeFirst Recorder, you agree that your contributions will be licensed under the [Apache License 2.0](LICENSE).

All source files must include the SPDX license header:

```rust
// Copyright 2025 Au-Zone Technologies Inc.
// SPDX-License-Identifier: Apache-2.0
```

## Questions?

- **Documentation**: https://doc.edgefirst.ai/latest/maivin/
- **Discussions**: https://github.com/EdgeFirstAI/recorder/discussions
- **Issues**: https://github.com/EdgeFirstAI/recorder/issues
- **Email**: support@au-zone.com

Thank you for helping make EdgeFirst Recorder better!
