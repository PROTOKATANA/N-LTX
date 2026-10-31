# Contributing to voltik

Issues and pull requests are welcome. Everyone taking part follows the [Code of Conduct](CODE_OF_CONDUCT.md).

## Reporting a problem

Open an issue with:

- the CircuiTikZ source, as small as still shows the problem;
- what voltik produced (the image, the warning or the error) and what TeX draws from the same source, if you have it;
- the version (`voltik --version`, or the crate, npm or PyPI version) and how you called it: CLI, Rust, Python or JavaScript.

A component voltik does not draw yet is not a bug; [`spec/subset.md`](../spec/subset.md) lists what is drawn and what is not. Ask for it in an issue if it is missing there.

Security problems go through [the security policy](SECURITY.md), not issues.

## Changing behavior

For a larger change (a new family of components, node-style components, voltage and current arrows), open an issue first, so the approach can be agreed on before the code.

The specification is the [CircuiTikZ manual](https://ctan.org/pkg/circuitikz) (version 1.8.7): shapes, measures and keys come from it, not from the CircuiTikZ source code, which voltik does not copy or read. When the manual leaves something open, measure it in what TeX draws with `tools/oracle/tex.sh`, and say in the pull request what was measured.

New features come with tests; [docs/tests.md](../docs/tests.md) lists what each test file covers. A new component also gets a circuit in `corpus/`, compared with TeX's drawing (`tools/oracle/side.py`, and `tools/oracle/ink.py` for labels). Run `tools/conformance/run.sh` and say in the pull request what moved in [docs/conformance.md](../docs/conformance.md).

Changes to the TikZ engine itself belong in [tikra](https://github.com/matheusmota-dev-br/tikra); voltik is built on `tikra::extension` and does not patch it.

Test inputs are made by hand for the test. Do not copy figures from books, exams or other material you do not hold the rights to, nor from the CircuiTikZ manual's examples verbatim.

## Before opening a pull request

Pull requests are not checked on GitHub; the checks run on your machine, through git hooks. Turn them on once after cloning:

```console
$ git config core.hooksPath .githooks
```

From then on, every commit runs fmt, clippy and the Rust tests, and every push the wasm, npm and Python tests, the same as `.github/workflows/ci.yml`. `scripts/check.sh` runs them all by hand, or one group: `scripts/check.sh rust`, `wasm` or `python`. They need rustup with the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` at the version in `Cargo.lock`, Node 22 and Python 3.12.

To work against a local checkout of tikra, see the Development section of the [README](../README.md); leave the `Cargo.lock` changes it causes out of commits.

- One change per pull request.
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/): `feat(core): …`, `fix(cli): …`, `docs: …`.
- Add what changed for users to `## Unreleased` in [CHANGELOG.md](../CHANGELOG.md) (create the section if it is not there).
- Leave versions alone; they are bumped in release pull requests.

Every pull request needs the approval of the maintainer before merging. After the merge, `.github/workflows/ci.yml` runs the same checks again on `main`.

## License

voltik is licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE), at your option. Unless you explicitly state otherwise, any contribution you intentionally submit for inclusion in voltik, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
