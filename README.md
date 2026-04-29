# s7cmd

Reliable, flexible, and fast command-line tool for Amazon S3

## Usage

```
Usage: s7cmd [OPTIONS] [COMMAND]

Object Operations:
  ls                                    List S3 objects
  cp                                    Copy objects from/to S3
  mv                                    Move objects from/to S3 (copy then delete source)
  rm                                    Delete a single S3 object
  sync                                  Synchronize files between local and S3 (or S3 to S3)
  clean                                 Bulk-delete S3 objects

Object Metadata:
  head-object                           Head an S3 object
  get-object-tagging                    Get an S3 object's tagging
  put-object-tagging                    Put tagging on an S3 object
  delete-object-tagging                 Delete tagging from an S3 object

Bucket Operations:
  create-bucket                         Create an S3 bucket
  delete-bucket                         Delete an S3 bucket
  head-bucket                           Head an S3 bucket

Bucket Tagging:
  get-bucket-tagging                    Get a bucket's tagging
  put-bucket-tagging                    Put tagging on a bucket
  delete-bucket-tagging                 Delete tagging from a bucket

Bucket Policy:
  get-bucket-policy                     Get a bucket's policy
  put-bucket-policy                     Put a bucket policy
  delete-bucket-policy                  Delete a bucket's policy

Bucket Versioning:
  get-bucket-versioning                 Get a bucket's versioning configuration
  put-bucket-versioning                 Put a bucket versioning configuration

Bucket Lifecycle:
  get-bucket-lifecycle-configuration    Get a bucket's lifecycle configuration
  put-bucket-lifecycle-configuration    Put a bucket lifecycle configuration
  delete-bucket-lifecycle-configuration Delete a bucket's lifecycle configuration

Bucket Encryption:
  get-bucket-encryption                 Get a bucket's encryption configuration
  put-bucket-encryption                 Put a bucket encryption configuration
  delete-bucket-encryption              Delete a bucket's encryption configuration

Bucket CORS:
  get-bucket-cors                       Get a bucket's CORS configuration
  put-bucket-cors                       Put a bucket CORS configuration
  delete-bucket-cors                    Delete a bucket's CORS configuration

Bucket Public Access Block:
  get-public-access-block               Get a bucket's public access block configuration
  put-public-access-block               Put a bucket public access block configuration
  delete-public-access-block            Delete a bucket's public access block configuration

Bucket Website:
  get-bucket-website                    Get a bucket's website configuration
  put-bucket-website                    Put a bucket website configuration
  delete-bucket-website                 Delete a bucket's website configuration

Bucket Logging:
  get-bucket-logging                    Get a bucket's logging configuration
  put-bucket-logging                    Put a bucket logging configuration

Bucket Notification:
  get-bucket-notification-configuration Get a bucket's notification configuration
  put-bucket-notification-configuration Put a bucket notification configuration

Other:
  help                                  Print this message or the help of the given subcommand(s)

Options:
      --auto-complete-shell <SHELL>  Generate shell completions for s7cmd (all subcommands) and exit [possible values: bash, elvish, fish, powershell, zsh]
  -h, --help                         Print help (see more with '--help')
  -V, --version                      Print version
```

## Documentation

For details on how to use these tools, please refer to the respective
pages—`ls` on [s3ls-rs](https://github.com/nidor1998/s3ls-rs), `sync` on
[s3sync](https://github.com/nidor1998/s3sync), `clean` on
[s3rm-rs](https://github.com/nidor1998/s3rm-rs), and others on
[s3util-rs](https://github.com/nidor1998/s3util-rs)—via the links
provided.

## Install

```bash
cargo install s7cmd
```

## License

Apache-2.0. See `LICENSE`.
