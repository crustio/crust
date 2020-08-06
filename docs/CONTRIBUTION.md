# Contribution

The `crust` project is an **OPENISH Open Source Project**

## What?

Individuals making significant and valuable contributions are given commit-access to a project to contribute as they see fit. A project is more like an open wiki than a standard guarded open source project.

## Rules

There are a few basic ground-rules for contributors (including the maintainer(s) of the project):

1. **No `--force` pushes** or modifying the master branch history in any way. If you need to rebase, ensure you do it in your own repo.
2. **Non-master branches**, prefixed with a short name moniker (e.g. zik/my-feature) must be used for ongoing work.
3. **All modifications** must be made in **pull-request** to solicit feedback from other contributors.
4. A pull-request **must not be merged until CI** has finished successfully.
5. Contributors should adhere to the substrate [House Coding Style](https://github.com/paritytech/substrate/blob/master/docs/STYLE_GUIDE.md).

## Merge process

##### In General

A PR needs to be reviewed and approved by project maintainers unless:

- it does not alter any logic (e.g. comments, dependencies, docs), then it may be tagged [`A5-insubstantial`](https://github.com/crustio/crust/labels/A5-insubstantial) and merged by its author once CI is complete.
- hotfix with no large change to logic, then it may be merged after a non-author contributor has approved the review once CI is complete.  

##### Label TL;DR

- `A-*` Pull request. ONE REQUIRED.
- `P-*` Priority. ONE REQUIRED.

##### Process

1. Please tag each *PR* with 1 `A` and `P` label at the minumum.
2. *PRs* that break the external API must be tagged with [`A2-breakapi`](https://github.com/crustio/crust/labels/A2-breakapi).
3. *PRs* that change the FRAME or consensus of running system with [`A0-breakconsensus`](https://github.com/crustio/crust/labels/A0-breakconsensus).
4. *PRs* that change the client or any other logic which may lead to hard fork must be tagged with [`A1-maybeforked`](https://github.com/crustio/crust/labels/A1-maybeforked).
5. PRs should be labeled with priority via the `P0-P2`.

## Issues

Please label issues with the following labels:

- `I-*` Issue severity and type. EXACTLY ONE REQUIRED.
- `P-*` Priority. AT MOST ONE ALLOWED.