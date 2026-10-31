# Security policy

## Supported versions

Fixes go into the latest release of each package: the crates `mathra` and `mathra-cli`, `@mathlabs/mathra` on npm and `mathra` on PyPI. Update to it before reporting.

## What counts

mathra is meant to read input it does not trust, such as equations typed by users of a site. These are security problems:

- an input that crashes the process (a panic that escapes the API), hangs it or makes it use time or memory far out of proportion to its size;
- an SVG or MathML output that carries something the input should not be able to put there: scripts, event handlers, external references, or markup that breaks out of the element;
- anything in the npm or Python packages that runs or reads more than the conversion needs.

A wrong drawing or a rejected equation is a bug: open an issue for it.

## Reporting

Report privately through the repository's **Security** tab → **[Report a vulnerability](https://github.com/matheusmota-dev-br/mathra/security/advisories/new)**. Do not open a public issue.

Include the input, the version, how you called mathra (CLI, Rust, Python or JavaScript) and what happened.

mathra is maintained by one person. You can expect an answer within a week; once the problem is confirmed, the fix is released as a patch version and the advisory is published with credit to you, unless you prefer otherwise.
