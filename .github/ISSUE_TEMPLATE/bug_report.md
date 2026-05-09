---
name: Bug report
about: Report a bug in s7cmd
title: "[Bug] "
labels: bug
---

## Contributing

- Bug reports are welcome, but responses are not guaranteed.
- Since this project is considered functionally complete, I will not accept any feature requests.
- If you find this project useful, feel free to fork and modify it as you wish.

🔒 I consider this project "complete" and will maintain it only minimally going forward.
However, I intend to keep the AWS SDK for Rust and other dependencies up to date monthly.

## Before opening an issue

Please read the [Scope and Non-Goals](https://github.com/nidor1998/s7cmd/blob/main/README.md#scope) and [Maintenance Model](https://github.com/nidor1998/s7cmd/blob/main/README.md#maintenance-model) sections of the README first. **Issues asking about behavior already documented in the README — including anything listed under Non-Goals — will be closed without further discussion.** This is not a rejection of your input; it is the project's documented scope and maintenance posture.

This template is for bug reports only. **Anything that is not a clear, reproducible bug in s7cmd itself — including feature requests, questions, usage help, configuration help, and discussion threads — will be closed unconditionally.**

**This project is intended for users who can self-resolve usage questions, configuration issues, and environment-specific problems. Only clear, reproducible bugs in s7cmd itself are accepted — nothing else.** No support, no questions, no feature requests, no usage help.

## Issue lifecycle

Issues with no activity for 30 days are labeled `stale` and closed 7 days later unless a new comment is added. Items labeled `pinned` or `security` are exempt. Closed issues can always be reopened.

## Prerequisites

**Issues without all of the following boxes checked will be closed without review.** No exceptions.

- [ ] I have read the README (including Scope and Non-Goals) and confirmed this issue is not already documented.
- [ ] I have searched existing issues (open and closed) and this is not a duplicate.
- [ ] This is a reproducible bug report — not a feature request, question, or request for usage help.
- [ ] I have reproduced this on the latest release of s7cmd.

## Describe the bug

A clear and concise description of what the bug is.

## To Reproduce

Please include as much of the information about the failed command as possible.

## Expected behavior

A clear and concise description of what you expected to happen.

## Environment

When verifying the reproducibility of a bug report, any report lacking information on the OS, s7cmd version, Storage, and Region will be closed without exception.

- OS: [e.g. macOS 14.5, Ubuntu 24.04, Windows 11]
- s7cmd version: [output of `s7cmd --version`] — **only the latest release is supported**. Issues filed against any other version will be closed automatically; please reproduce on the latest version before filing.
- Storage: [e.g., Amazon S3, MinIO, Cloudflare R2, Ceph RGW] — only Amazon S3 is supported. Issues regarding S3-compatible services will be closed automatically, unconditionally, and without exception.
- Region: [e.g., us-east-1, ap-northeast-1]

## Additional context

Add any other context about the problem here.
