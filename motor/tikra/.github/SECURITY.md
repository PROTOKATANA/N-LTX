# Security policy

## Supported versions

Fixes go into the latest release of each package: the crates `tikra` and `tikra-cli`, `@mathlabs/tikra` on npm and `tikra` on PyPI. Update to it before reporting.

## What counts

tikra is meant to read input it does not trust, such as figures written by users of a site. TikZ is a language with macros and loops, so these are security problems:

- an input that crashes the process (a panic that escapes the API), hangs it or makes it use time or memory far out of proportion to its size, for example through recursive macros, endless loops or huge coordinates;
- an SVG output that carries something the input should not be able to put there: scripts, event handlers, external references, or markup that breaks out of the element;
- an input that makes tikra read or write files, or reach the network;
- anything in the npm or Python packages that runs or reads more than the conversion needs.

A wrong drawing or an unsupported command is a bug: open an issue for it.

## Reporting

Report privately through the repository's **Security** tab → **[Report a vulnerability](https://github.com/matheusmota-dev-br/tikra/security/advisories/new)**. Do not open a public issue.

Include the source, the version, how you called tikra (CLI, Rust, Python or JavaScript) and what happened.

tikra is maintained by one person. You can expect an answer within a week; once the problem is confirmed, the fix is released as a patch version and the advisory is published with credit to you, unless you prefer otherwise.
