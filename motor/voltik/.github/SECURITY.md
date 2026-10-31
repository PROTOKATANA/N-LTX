# Security policy

## Supported versions

Fixes go into the latest release of each package: the crates `voltik` and `voltik-cli`, `@mathlabs/voltik` on npm and `voltik` on PyPI. Update to it before reporting.

## What counts

voltik is meant to read input it does not trust, such as circuits written by users of a site. It reads CircuiTikZ through tikra, and TikZ is a language with macros and loops, so these are security problems:

- an input that crashes the process (a panic that escapes the API), hangs it or makes it use time or memory far out of proportion to its size, for example through `\ctikzset` values, component options or huge coordinates;
- an SVG output that carries something the input should not be able to put there: scripts, event handlers, external references, or markup that breaks out of the element;
- an input that makes voltik read or write files, or reach the network;
- anything in the npm or Python packages that runs or reads more than the conversion needs.

A wrong drawing or a component not drawn yet is a bug: open an issue for it. A problem in the TikZ engine itself, with no CircuiTikZ in the input, goes to [tikra's security policy](https://github.com/matheusmota-dev-br/tikra/security/policy).

## Reporting

Report privately through the repository's **Security** tab → **[Report a vulnerability](https://github.com/matheusmota-dev-br/voltik/security/advisories/new)**. Do not open a public issue.

Include the source, the version, how you called voltik (CLI, Rust, Python or JavaScript) and what happened.

voltik is maintained by one person. You can expect an answer within a week; once the problem is confirmed, the fix is released as a patch version and the advisory is published with credit to you, unless you prefer otherwise.
