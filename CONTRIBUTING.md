# Contributing

Thanks for your interest in contributing to magpie.

## Reporting issues

Use the issue templates for bug reports and feature requests. Search existing issues before opening a new one.

## Development

1. Install a stable Rust toolchain and [pre-commit](https://pre-commit.com/).
2. Run `pre-commit install` once after cloning.
3. Build and test with `cargo build` and `cargo test`.

## Pull requests

1. Fork the repository and create a branch from `master`.
2. Keep changes focused. One logical change per pull request.
3. Add or update tests and documentation.
4. Ensure `pre-commit run --all-files` and CI pass.
5. Fill out the pull request template.

## Commit messages

This project follows [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).

```
feat(pypi): serve PEP 658 metadata files
fix(db): handle normalized names in lookups
```

Common types: `feat`, `fix`, `docs`, `refactor`, `test`, `build`, `ci`, `chore`. Mark breaking changes with `!` or a `BREAKING CHANGE:` footer.
