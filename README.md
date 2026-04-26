# s7cmd

Unified S3 CLI bundling [s3sync](https://github.com/nidor1998/s3sync) and
[s3util-rs](https://github.com/nidor1998/s3util-rs) into a single binary.

## Subcommands

| Subcommand | What it does | Source library |
|---|---|---|
| `sync` | Synchronize a directory tree between local and S3 | s3sync |
| `cp` / `mv` / `rm` | Copy / move / delete objects | s3util-rs |
| `create-bucket` / `delete-bucket` / `head-bucket` | Bucket lifecycle | s3util-rs |
| `head-object` | Object metadata (HEAD) | s3util-rs |
| `get-object-tagging` / `put-object-tagging` / `delete-object-tagging` | Object tagging | s3util-rs |
| `get-bucket-tagging` / `put-bucket-tagging` / `delete-bucket-tagging` | Bucket tagging | s3util-rs |
| `get-bucket-policy` / `put-bucket-policy` / `delete-bucket-policy` | Bucket policy | s3util-rs |
| `get-bucket-versioning` / `put-bucket-versioning` | Bucket versioning | s3util-rs |

Each subcommand accepts the same flags as its standalone equivalent
(`s3sync ...` for `sync`; `s3util <cmd> ...` for the rest). See
`s7cmd <subcommand> --help` for the full flag reference.

## Install

```bash
cargo install s7cmd
```

## Lua scripting

`s7cmd sync` includes s3sync's Lua scripting passthrough by default
(`--filter-callback-lua-script`, `--event-callback-lua-script`,
`--preprocess-callback-lua-script`, etc.). To opt out, build with
`--no-default-features` after re-pinning s3sync with
`default-features = false` in your Cargo.toml.

## License

Apache-2.0. See `LICENSE` and `ATTRIBUTIONS.md` for the upstream sources
that s7cmd vendors.
