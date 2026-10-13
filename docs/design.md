# Goals

## Target Environment

Primary targets are `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl`, deployable on an Alpine or scratch base image. Enterprise environments commonly run docker, docker-compose, docker-swarm, kubernetes, podman, etc., so lightweight images matter. Ubuntu and Debian are secondary targets, Windows is tertiary. Mac builds are not prioritized given the cost of build and test environments.

Decisions:
- Use a different global allocator. musl's malloc is slow in threaded environments.
- Use reqwest's `hickory-dns` feature. musl's DNS resolver behaves oddly in Kubernetes due to its strict standards compliance, parallel query design, and Kubernetes' defaults.
- Avoid OpenSSL entirely in favor of rustls.

## Target Features

Feature set similar to Kellnr, accounting for differences between the Python and Rust ecosystems:

- Hosted repos plus the legacy upload API. Support twine, uv publish, poetry, hatch, and flit.
- Proxy repos for pypi.org and other upstreams.
- Documentation hosting.
- Server-rendered UI with minimal JavaScript.
- SQLite and Postgres support.
- Filesystem, S3, and GCS support.
- Robust user, group, and permission systems.
- Trivial containerization.
- Configuration via command line, then environment, then TOML.

Extension features:
- OIDC SSO
- Protection against common dependency confusion attacks
- Serve cached data when upstream is down
- Multiple upstreams
- Per-upstream credentials
- `HTTP_PROXY` / `NO_PROXY` support
- Custom CA bundle
- Prometheus metrics

# Implementation

## Protocol Surface

- [PEP 503](https://peps.python.org/pep-0503/): HTML simple API
- [PEP 691](https://peps.python.org/pep-0691/): JSON simple API
- [PEP 658](https://peps.python.org/pep-0658/): Serve distribution metadata in the simple repository API
- [PEP 714](https://peps.python.org/pep-0714/): Rename dist-info-metadata in the simple API
- [PEP 592](https://peps.python.org/pep-0592/): Adding "Yank" support to the Simple API
- [PEP 700](https://peps.python.org/pep-0700/): Additional fields for the simple API for package indexes
- [PyPI JSON API](https://docs.pypi.org/api/json/): Parity with mainline repositories
- [PEP 427](https://peps.python.org/pep-0427/): The Wheel Binary Package Format 1.0
- [PEP 625](https://peps.python.org/pep-0625/): Filename of a Source Distribution
- [PEP 740](https://peps.python.org/pep-0740/): Index support for digital attestations

## Dependency Stack

- HTTP: [axum](https://crates.io/crates/axum), [hyper](https://crates.io/crates/hyper), [tokio](https://crates.io/crates/tokio), [tower](https://crates.io/crates/tower), [tower-http](https://crates.io/crates/tower-http)
- Outbound requests: [reqwest](https://crates.io/crates/reqwest)
- TLS: [rustls](https://crates.io/crates/rustls)
- Database: [sqlx](https://crates.io/crates/sqlx). Target the SQL subset common to SQLite and Postgres.
- Storage: [object_store](https://crates.io/crates/object_store)
- Cache: [moka](https://crates.io/crates/moka)
- Logging: [tracing](https://crates.io/crates/tracing)
- OIDC: [openidconnect](https://crates.io/crates/openidconnect)
- LDAP: [ldap3](https://crates.io/crates/ldap3)
- OAuth2: [dex](https://crates.io/crates/dex)
- Security: [argon2](https://crates.io/crates/argon2)
