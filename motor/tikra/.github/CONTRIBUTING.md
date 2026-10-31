# Contributing to tikra

Issues and pull requests are welcome. Everyone taking part follows the [Code of Conduct](CODE_OF_CONDUCT.md).

## Reporting a problem

Open an issue with:

- the TikZ or pgfplots source, as small as still shows the problem;
- what tikra produced (the image, the warning or the error) and what TeX draws from the same source, if you have it;
- the version (`tikra --version`, or the crate, npm or PyPI version) and how you called it: CLI, Rust, Python or JavaScript.

Security problems go through [the security policy](SECURITY.md), not issues.

## Changing behavior

For a larger change (a new library, a new kind of path or plot), open an issue first, so the approach can be agreed on before the code.

New features come with tests; [docs/tests.md](../docs/tests.md) lists what each test file covers. When the change concerns how close tikra gets to TikZ, run `tools/conformance/run.sh` and say in the pull request what moved.

Libraries that live outside tikra (for example, a set of circuit symbols) can be built on `tikra::extension` without changing the engine.

Test inputs are made by hand for the test. Do not copy figures from books, exams or other material you do not hold the rights to.

## Before opening a pull request

Pull requests are not checked on GitHub; the checks run on your machine, through git hooks. Turn them on once after cloning:

```console
$ git config core.hooksPath .githooks
```

From then on, every commit runs fmt, clippy and the Rust tests, and every push the wasm, npm and Python tests, the same as `.github/workflows/ci.yml`. `scripts/check.sh` runs them all by hand, or one group: `scripts/check.sh rust`, `wasm` or `python`. They need rustup with the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` at the version in `Cargo.lock`, Node 22 and Python 3.12.

Coverage is at 100% of lines (`cargo llvm-cov --workspace --summary-only`); keep it there.

To work against a local checkout of mathra, see the Development section of the [README](../README.md); leave the `Cargo.lock` changes it causes out of commits.

- One change per pull request.
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/): `feat(core): …`, `fix(cli): …`, `docs: …`.
- Add what changed for users to `## Unreleased` in [CHANGELOG.md](../CHANGELOG.md) (create the section if it is not there).
- Leave versions alone; they are bumped in release pull requests.

Every pull request needs the approval of the maintainer before merging. After the merge, `.github/workflows/ci.yml` runs the same checks again on `main`.

## License

tikra is licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE), at your option. Unless you explicitly state otherwise, any contribution you intentionally submit for inclusion in tikra, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
