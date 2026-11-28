//! Process-level regression tests for the `sync` fixes that arrived with
//! s3sync 1.64.1 (nidor1998/s3sync#267) and that need an S3 endpoint to
//! observe.
//!
//! The real binary runs against a small in-process S3 endpoint (loopback
//! only, no AWS access): each test serves a fixed set of objects, optionally
//! fails chosen requests, and then asserts on the exit code and on the
//! requests the binary actually sent (method, path, query and headers).
//!
//! Covered here:
//! - the prefix of an S3 path is used exactly as given;
//! - `--metadata` keeps the s3sync origin metadata;
//! - force retry applies to connection errors in multipart upload requests
//!   and `CopyObject`, and survives a failed `AbortMultipartUpload`;
//! - `--if-match` reaches single-part `CopyObject` requests;
//! - a failed ranged `GetObject` for a later part is an error, not a panic;
//! - `--check-etag --auto-chunksize` digests the whole file for a
//!   single-part object;
//! - a delete marker reaching the Lua filter callback, and a sync warning
//!   with `--enable-versioning`, do not panic.

mod common;

use common::{run, s7cmd_cmd_clean_env};
use std::collections::BTreeMap;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const MIB: usize = 1024 * 1024;
const LAST_MODIFIED_HTTP: &str = "Thu, 01 Jan 2026 00:00:00 GMT";
const LAST_MODIFIED_ISO: &str = "2026-01-01T00:00:00.000Z";
/// ETag returned for every write. Tests that write pass
/// `--disable-etag-verify`, so the value only has to be well-formed.
const WRITE_ETAG: &str = "\"00000000000000000000000000000000\"";

// ---------------------------------------------------------------
// Fake S3 endpoint
// ---------------------------------------------------------------

/// One request received by the fake endpoint.
#[derive(Clone, Debug)]
struct Request {
    method: String,
    /// Raw request target: percent-encoded path plus query.
    target: String,
    /// Header names are lowercased.
    headers: Vec<(String, String)>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    fn raw_path(&self) -> &str {
        self.target.split('?').next().unwrap_or("")
    }

    /// Percent-decoded path, e.g. `/bucket/dir/key`.
    fn path(&self) -> String {
        decode(self.raw_path())
    }

    fn raw_query(&self, name: &str) -> Option<&str> {
        let (_, query) = self.target.split_once('?')?;
        query.split('&').find_map(|param| {
            let (key, value) = param.split_once('=').unwrap_or((param, ""));
            (key == name).then_some(value)
        })
    }

    fn has_query(&self, name: &str) -> bool {
        self.raw_query(name).is_some()
    }

    /// Percent-decoded query parameter value.
    fn query(&self, name: &str) -> Option<String> {
        self.raw_query(name).map(decode)
    }

    fn is(&self, method: &str, operation: Operation) -> bool {
        self.method == method && operation.matches(self)
    }
}

/// The S3 operations the tests need to tell apart.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Operation {
    ListObjectsV2,
    CreateMultipartUpload,
    UploadPart,
    UploadPartCopy,
    CompleteMultipartUpload,
    AbortMultipartUpload,
    CopyObject,
    PutObject,
    GetObject,
}

impl Operation {
    fn matches(self, request: &Request) -> bool {
        let is_copy = request.header("x-amz-copy-source").is_some();
        let is_part = request.has_query("partNumber");
        match self {
            Self::ListObjectsV2 => request.has_query("list-type"),
            Self::CreateMultipartUpload => request.method == "POST" && request.has_query("uploads"),
            Self::UploadPart => request.method == "PUT" && is_part && !is_copy,
            Self::UploadPartCopy => request.method == "PUT" && is_part && is_copy,
            Self::CompleteMultipartUpload => {
                request.method == "POST" && request.has_query("uploadId")
            }
            Self::AbortMultipartUpload => {
                request.method == "DELETE" && request.has_query("uploadId")
            }
            Self::CopyObject => request.method == "PUT" && !is_part && is_copy,
            Self::PutObject => request.method == "PUT" && !is_part && !is_copy,
            Self::GetObject => {
                request.method == "GET" && request.raw_query("x-id") == Some("GetObject")
            }
        }
    }
}

fn decode(value: &str) -> String {
    urlencoding::decode(value)
        .expect("request target must be valid percent-encoded UTF-8")
        .into_owned()
}

/// What the endpoint does with one request.
enum Reply {
    Response {
        status: u16,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    },
    /// Read the request, then close the connection without answering. The
    /// SDK reports this as a dispatch failure (a connection error).
    CloseConnection,
}

impl Reply {
    fn xml(status: u16, body: impl Into<String>) -> Self {
        Self::Response {
            status,
            headers: vec![("content-type".to_string(), "application/xml".to_string())],
            body: body.into().into_bytes(),
        }
    }

    fn error(status: u16, code: &str) -> Self {
        Self::xml(
            status,
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Error><Code>{code}</Code><Message>fake {code}</Message></Error>"
            ),
        )
    }

    fn empty(status: u16, headers: &[(&str, &str)]) -> Self {
        Self::Response {
            status,
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            body: Vec::new(),
        }
    }
}

/// Handle to a fake endpoint running on background threads.
struct FakeS3 {
    endpoint: String,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl FakeS3 {
    /// Serve every request with `handler`, one thread per connection (the
    /// sync workers send requests concurrently).
    fn spawn(handler: impl Fn(&Request) -> Reply + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind fake S3 listener");
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let handler = Arc::new(handler);

        let recorded = Arc::clone(&requests);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { break };
                let handler = Arc::clone(&handler);
                let recorded = Arc::clone(&recorded);
                std::thread::spawn(move || serve_connection(stream, &*handler, &recorded));
            }
        });

        Self { endpoint, requests }
    }

    /// Serve `fixture` without failing anything.
    fn serve(fixture: Fixture) -> Self {
        Self::spawn(move |request| fixture.respond(request))
    }

    fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }

    fn requests_of(&self, method: &str, operation: Operation) -> Vec<Request> {
        self.requests()
            .into_iter()
            .filter(|request| request.is(method, operation))
            .collect()
    }

    fn count(&self, method: &str, operation: Operation) -> usize {
        self.requests_of(method, operation).len()
    }

    /// Request lines, for assertion messages.
    fn request_log(&self) -> String {
        self.requests()
            .iter()
            .map(|request| format!("{} {}", request.method, request.target))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn serve_connection(
    mut stream: TcpStream,
    handler: &(dyn Fn(&Request) -> Reply + Send + Sync),
    recorded: &Mutex<Vec<Request>>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let Some(request) = read_request(&mut stream) else {
        return;
    };
    recorded.lock().unwrap().push(request.clone());

    match handler(&request) {
        Reply::CloseConnection => {}
        Reply::Response {
            status,
            headers,
            body,
        } => {
            let mut head = format!("HTTP/1.1 {status} Fake\r\nconnection: close\r\n");
            if !headers.iter().any(|(k, _)| k == "content-length") {
                head.push_str(&format!("content-length: {}\r\n", body.len()));
            }
            for (name, value) in &headers {
                head.push_str(&format!("{name}: {value}\r\n"));
            }
            head.push_str("\r\n");
            let _ = stream.write_all(head.as_bytes());
            // A HEAD response carries the headers of the object but no body.
            if request.method != "HEAD" {
                let _ = stream.write_all(&body);
            }
            let _ = stream.flush();
        }
    }
}

/// Read one HTTP request, including its `content-length` body (which is
/// discarded: the tests assert on the request line and the headers).
fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut data = Vec::new();
    let mut buf = vec![0u8; 64 * 1024];
    let header_end = loop {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return None,
            Ok(n) => {
                data.extend_from_slice(&buf[..n]);
                if let Some(pos) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                    break pos + 4;
                }
            }
        }
    };

    let head = String::from_utf8_lossy(&data[..header_end]).to_string();
    let mut lines = head.split("\r\n");
    let mut request_line = lines.next()?.split_whitespace();
    let method = request_line.next()?.to_string();
    let target = request_line.next()?.to_string();
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_ascii_lowercase(), value.trim().to_string()))
        })
        .collect();
    let request = Request {
        method,
        target,
        headers,
    };

    if request
        .header("expect")
        .is_some_and(|value| value.eq_ignore_ascii_case("100-continue"))
    {
        let _ = stream.write_all(b"HTTP/1.1 100 Continue\r\n\r\n");
    }

    let content_length: usize = request
        .header("content-length")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let mut remaining = content_length.saturating_sub(data.len() - header_end);
    while remaining > 0 {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => remaining = remaining.saturating_sub(n),
        }
    }

    Some(request)
}

// ---------------------------------------------------------------
// Canned bucket contents
// ---------------------------------------------------------------

/// An object served by the fake endpoint. Its body is `size` copies of
/// `fill`, generated on demand.
#[derive(Clone)]
struct FakeObject {
    size: usize,
    fill: u8,
    /// Quoted ETag, as S3 returns it.
    e_tag: String,
    version_id: Option<String>,
    /// `LastModified` reported by `ListObjectsV2`.
    last_modified: &'static str,
}

impl FakeObject {
    fn new(size: usize) -> Self {
        let fill = b'x';
        Self {
            size,
            fill,
            e_tag: format!("\"{:x}\"", md5::compute(vec![fill; size])),
            version_id: None,
            last_modified: LAST_MODIFIED_ISO,
        }
    }

    /// Make the object older than the default, so that a source object with
    /// the default modification time is newer and gets synced over it.
    fn older(mut self) -> Self {
        self.last_modified = "2020-01-01T00:00:00.000Z";
        self
    }

    fn with_e_tag(mut self, e_tag: &str) -> Self {
        self.e_tag = e_tag.to_string();
        self
    }

    fn with_version_id(mut self, version_id: &str) -> Self {
        self.version_id = Some(version_id.to_string());
        self
    }
}

#[derive(Clone, Default)]
struct FakeBucket {
    objects: BTreeMap<String, FakeObject>,
    versioning_enabled: bool,
    /// `ListObjectVersions` response body for the whole bucket. A listing
    /// with a non-empty `prefix` (the per-key target lookup) is always empty.
    versions_xml: Option<String>,
}

#[derive(Clone, Default)]
struct Fixture {
    buckets: BTreeMap<String, FakeBucket>,
}

impl Fixture {
    fn bucket(mut self, name: &str, bucket: FakeBucket) -> Self {
        self.buckets.insert(name.to_string(), bucket);
        self
    }

    /// Answer `request` like S3 would for the configured buckets. Any
    /// bucket that is not configured behaves as an empty bucket.
    fn respond(&self, request: &Request) -> Reply {
        let path = request.path();
        let (bucket_name, key) = path
            .trim_start_matches('/')
            .split_once('/')
            .unwrap_or((path.trim_start_matches('/'), ""));
        let empty_bucket = FakeBucket::default();
        let bucket = self.buckets.get(bucket_name).unwrap_or(&empty_bucket);

        match request.method.as_str() {
            "GET" if request.has_query("versioning") => {
                let status = if bucket.versioning_enabled {
                    "<Status>Enabled</Status>"
                } else {
                    ""
                };
                Reply::xml(
                    200,
                    format!(
                        r#"<?xml version="1.0" encoding="UTF-8"?>
<VersioningConfiguration xmlns="http://s3.amazonaws.com/doc/2006-03-01/">{status}</VersioningConfiguration>"#
                    ),
                )
            }
            "GET" if request.has_query("versions") => {
                let whole_bucket = request.query("prefix").unwrap_or_default().is_empty();
                match (&bucket.versions_xml, whole_bucket) {
                    (Some(versions_xml), true) => Reply::xml(200, versions_xml.clone()),
                    _ => Reply::xml(200, empty_versions_page(bucket_name)),
                }
            }
            "GET" if request.has_query("list-type") => {
                let prefix = request.query("prefix").unwrap_or_default();
                Reply::xml(200, objects_page(bucket_name, bucket, &prefix))
            }
            "GET" | "HEAD" => match bucket.objects.get(key) {
                Some(object) => object_reply(object, request),
                None => Reply::error(404, "NoSuchKey"),
            },
            "POST" if request.has_query("uploads") => Reply::xml(
                200,
                format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<InitiateMultipartUploadResult><Bucket>{bucket_name}</Bucket><Key>{key}</Key><UploadId>fake-upload-id</UploadId></InitiateMultipartUploadResult>"#
                ),
            ),
            "POST" if request.has_query("uploadId") => Reply::xml(
                200,
                format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<CompleteMultipartUploadResult><Bucket>{bucket_name}</Bucket><Key>{key}</Key><ETag>&quot;00000000000000000000000000000000-2&quot;</ETag></CompleteMultipartUploadResult>"#
                ),
            ),
            "PUT" if Operation::UploadPartCopy.matches(request) => Reply::xml(
                200,
                r#"<?xml version="1.0" encoding="UTF-8"?>
<CopyPartResult><ETag>&quot;00000000000000000000000000000000&quot;</ETag><LastModified>2026-01-01T00:00:00.000Z</LastModified></CopyPartResult>"#,
            ),
            "PUT" if Operation::CopyObject.matches(request) => Reply::xml(
                200,
                r#"<?xml version="1.0" encoding="UTF-8"?>
<CopyObjectResult><ETag>&quot;00000000000000000000000000000000&quot;</ETag><LastModified>2026-01-01T00:00:00.000Z</LastModified></CopyObjectResult>"#,
            ),
            "PUT" => Reply::empty(200, &[("etag", WRITE_ETAG)]),
            "DELETE" => Reply::empty(204, &[]),
            _ => Reply::error(400, "BadRequest"),
        }
    }
}

fn empty_versions_page(bucket_name: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ListVersionsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Name>{bucket_name}</Name><Prefix></Prefix><MaxKeys>1000</MaxKeys><IsTruncated>false</IsTruncated></ListVersionsResult>"#
    )
}

fn objects_page(bucket_name: &str, bucket: &FakeBucket, prefix: &str) -> String {
    let mut contents = String::new();
    for (key, object) in bucket
        .objects
        .iter()
        .filter(|(key, _)| key.starts_with(prefix))
    {
        contents.push_str(&format!(
            "<Contents><Key>{key}</Key><LastModified>{}</LastModified><ETag>{}</ETag><Size>{}</Size><StorageClass>STANDARD</StorageClass></Contents>",
            object.last_modified,
            object.e_tag.replace('"', "&quot;"),
            object.size,
        ));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Name>{bucket_name}</Name><MaxKeys>1000</MaxKeys><IsTruncated>false</IsTruncated>{contents}</ListBucketResult>"#
    )
}

/// `GetObject` / `HeadObject`, honoring `Range`. A `partNumber` request is
/// answered like S3 answers it for an object uploaded in a single part: the
/// whole object, without a parts count.
fn object_reply(object: &FakeObject, request: &Request) -> Reply {
    let (mut start, mut end) = (0usize, object.size.saturating_sub(1));
    let mut status = 200;
    let mut headers = vec![
        (
            "content-type".to_string(),
            "application/octet-stream".to_string(),
        ),
        ("etag".to_string(), object.e_tag.clone()),
        ("last-modified".to_string(), LAST_MODIFIED_HTTP.to_string()),
        ("accept-ranges".to_string(), "bytes".to_string()),
    ];
    if let Some(version_id) = &object.version_id {
        headers.push(("x-amz-version-id".to_string(), version_id.clone()));
    }

    if let Some(range) = request.header("range") {
        let (range_start, range_end) = range
            .trim_start_matches("bytes=")
            .split_once('-')
            .expect("range header must be bytes=start-end");
        start = range_start.parse().expect("range start");
        end = range_end.parse().unwrap_or(end);
        status = 206;
    } else if request.has_query("partNumber") {
        status = 206;
    }
    let length = if object.size == 0 { 0 } else { end - start + 1 };
    if status == 206 {
        headers.push((
            "content-range".to_string(),
            format!("bytes {start}-{end}/{}", object.size),
        ));
    }
    headers.push(("content-length".to_string(), length.to_string()));

    Reply::Response {
        status,
        headers,
        body: vec![object.fill; length],
    }
}

/// Fails the first `count` requests it is asked about, then lets the rest
/// through.
struct FailFirst(AtomicUsize);

impl FailFirst {
    fn new(count: usize) -> Self {
        Self(AtomicUsize::new(count))
    }

    fn should_fail(&self) -> bool {
        self.0
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |remaining| {
                remaining.checked_sub(1)
            })
            .is_ok()
    }
}

// ---------------------------------------------------------------
// Running s7cmd
// ---------------------------------------------------------------

/// Client flags for one side (`source` / `target`) pointing at the fake
/// endpoint with static credentials, so no profile, IMDS or config-file
/// lookup happens.
fn client_args(side: &str, endpoint: &str) -> Vec<String> {
    vec![
        format!("--{side}-endpoint-url"),
        endpoint.to_string(),
        format!("--{side}-access-key"),
        "fake-access-key".to_string(),
        format!("--{side}-secret-access-key"),
        "fake-secret-key".to_string(),
        format!("--{side}-region"),
        "us-east-1".to_string(),
    ]
}

/// Run `s7cmd sync` with the SDK's own retries disabled, so every request
/// the fake endpoint records was sent by the sync pipeline itself.
fn run_sync(args: Vec<String>) -> (Option<i32>, String, String) {
    let mut cmd = s7cmd_cmd_clean_env();
    cmd.env("AWS_EC2_METADATA_DISABLED", "true")
        .arg("sync")
        .args(["--aws-max-attempts", "1"])
        .args(args);
    run(&mut cmd)
}

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| arg.to_string()).collect()
}

/// `dir` as a sync directory argument (with a trailing separator).
fn dir_arg(dir: &Path) -> String {
    format!("{}/", dir.to_str().unwrap())
}

fn assert_no_panic(stdout: &str, stderr: &str) {
    assert!(
        !stdout.contains("panicked") && !stderr.contains("panicked"),
        "s7cmd must not panic; stdout={stdout}\nstderr={stderr}"
    );
}

/// A local directory holding one file large enough to be uploaded in two
/// parts with a 5MiB threshold and chunk size.
fn two_part_source_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("big.dat"), vec![b'x'; 5 * MIB + 1024]).unwrap();
    dir
}

const TWO_PART_UPLOAD_ARGS: [&str; 9] = [
    "--multipart-threshold",
    "5MiB",
    "--multipart-chunksize",
    "5MiB",
    "--force-retry-count",
    "2",
    "--force-retry-interval-milliseconds",
    "1",
    "--disable-etag-verify",
];

// ---------------------------------------------------------------
// S3 prefix is used exactly as given
// ---------------------------------------------------------------

/// The prefix of the source path must reach `ListObjectsV2` unchanged:
/// `?` and `#` are part of it, percent escapes are not decoded, and `.` /
/// `..` segments are not collapsed. Before s3sync 1.64.1 the path was
/// parsed as a URL, so e.g. `data#old/` listed `data`.
#[test]
fn source_prefix_is_listed_exactly_as_given() {
    let fake = FakeS3::serve(Fixture::default());
    let local_dir = tempfile::tempdir().unwrap();

    let prefixes = [
        "data#old/",
        "data?old/",
        "data#",
        "data?",
        "a?b#c?d#e/",
        "#top",
        "?top",
        "a%23b%3Fc#d",
        "%FF",
        "100%",
        "a%20b c/",
        "a/../b/",
        "./a/./b/",
        "a?x/../y#%2e%2e/z",
        "dir/こんにちは#☃?x y/",
        "/",
        "",
    ];

    for prefix in prefixes {
        let mut args = strings(&["--dry-run"]);
        args.extend(client_args("source", &fake.endpoint));
        args.push(format!("s3://fake-bucket/{prefix}"));
        args.push(dir_arg(local_dir.path()));
        let (code, stdout, stderr) = run_sync(args);
        assert_eq!(
            code,
            Some(0),
            "prefix {prefix:?} must be accepted; stdout={stdout}\nstderr={stderr}"
        );

        let listings = fake.requests_of("GET", Operation::ListObjectsV2);
        let listing = listings.last().expect("a ListObjectsV2 request");
        assert_eq!(listing.path(), "/fake-bucket/", "prefix {prefix:?}");
        assert_eq!(
            listing.query("prefix").as_deref(),
            Some(prefix),
            "prefix {prefix:?} must be sent unchanged; request: {}",
            listing.target
        );
    }

    assert_eq!(
        fake.count("GET", Operation::ListObjectsV2),
        prefixes.len(),
        "one listing per run; requests:\n{}",
        fake.request_log()
    );
}

/// The prefix of the target path must be the literal start of every
/// uploaded key.
#[test]
fn target_prefix_is_used_exactly_as_given_for_uploaded_keys() {
    let fake = FakeS3::serve(Fixture::default());
    let source_dir = tempfile::tempdir().unwrap();
    std::fs::write(source_dir.path().join("file.txt"), b"data").unwrap();

    for prefix in ["data#old?x/", "a%20b c/", "%FF/", "?top/"] {
        let mut args = strings(&["--disable-etag-verify"]);
        args.extend(client_args("target", &fake.endpoint));
        args.push(dir_arg(source_dir.path()));
        args.push(format!("s3://fake-bucket/{prefix}"));
        let (code, stdout, stderr) = run_sync(args);
        assert_eq!(
            code,
            Some(0),
            "prefix {prefix:?}: stdout={stdout}\nstderr={stderr}"
        );

        let puts = fake.requests_of("PUT", Operation::PutObject);
        let put = puts.last().expect("a PutObject request");
        assert_eq!(
            put.path(),
            format!("/fake-bucket/{prefix}file.txt"),
            "prefix {prefix:?}; request: {}",
            put.target
        );
        // Nothing of the prefix may leak into the query string.
        assert_eq!(
            put.target.split_once('?').map(|(_, query)| query),
            Some("x-id=PutObject"),
            "prefix {prefix:?}; request: {}",
            put.target
        );
    }
}

// ---------------------------------------------------------------
// --metadata keeps the s3sync origin metadata
// ---------------------------------------------------------------

fn upload_one_file_and_get_put_request(extra_args: &[&str]) -> Request {
    let fake = FakeS3::serve(Fixture::default());
    let source_dir = tempfile::tempdir().unwrap();
    std::fs::write(source_dir.path().join("file.txt"), b"data").unwrap();

    let mut args = strings(extra_args);
    args.push("--disable-etag-verify".to_string());
    args.extend(client_args("target", &fake.endpoint));
    args.push(dir_arg(source_dir.path()));
    args.push("s3://fake-bucket/".to_string());
    let (code, stdout, stderr) = run_sync(args);
    assert_eq!(code, Some(0), "stdout={stdout}\nstderr={stderr}");

    let mut puts = fake.requests_of("PUT", Operation::PutObject);
    assert_eq!(puts.len(), 1, "requests:\n{}", fake.request_log());
    puts.remove(0)
}

/// `--metadata` used to replace the whole metadata map after the origin
/// metadata had been added, so `--put-last-modified-metadata` was lost.
#[test]
fn metadata_option_keeps_last_modified_origin_metadata() {
    let put = upload_one_file_and_get_put_request(&[
        "--metadata",
        "key1=value1",
        "--put-last-modified-metadata",
    ]);

    assert_eq!(put.header("x-amz-meta-key1"), Some("value1"));
    assert!(
        put.header("x-amz-meta-s3sync_origin_last_modified")
            .is_some_and(|value| !value.is_empty()),
        "origin last-modified metadata is missing; headers: {:?}",
        put.headers
    );
}

/// `--metadata` alone must not invent origin metadata.
#[test]
fn metadata_option_alone_adds_no_origin_metadata() {
    let put = upload_one_file_and_get_put_request(&["--metadata", "key1=value1"]);

    assert_eq!(put.header("x-amz-meta-key1"), Some("value1"));
    assert!(
        put.header("x-amz-meta-s3sync_origin_last_modified")
            .is_none()
    );
    assert!(put.header("x-amz-meta-s3sync_origin_version_id").is_none());
}

fn versions_page(entries: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ListVersionsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Name>source-bucket</Name><Prefix></Prefix><MaxKeys>1000</MaxKeys><IsTruncated>false</IsTruncated>{entries}</ListVersionsResult>"#
    )
}

fn version_entry(key: &str, version_id: &str, is_latest: bool) -> String {
    format!(
        "<Version><Key>{key}</Key><VersionId>{version_id}</VersionId><IsLatest>{is_latest}</IsLatest><LastModified>{LAST_MODIFIED_ISO}</LastModified><ETag>&quot;9dd4e461268c8034f5c8564e155c67a6&quot;</ETag><Size>1</Size><StorageClass>STANDARD</StorageClass></Version>"
    )
}

fn delete_marker_entry(key: &str, version_id: &str) -> String {
    format!(
        "<DeleteMarker><Key>{key}</Key><VersionId>{version_id}</VersionId><IsLatest>true</IsLatest><LastModified>2026-01-02T00:00:00.000Z</LastModified></DeleteMarker>"
    )
}

fn versioned_bucket(versions_xml: String, objects: &[(&str, &str)]) -> FakeBucket {
    FakeBucket {
        objects: objects
            .iter()
            .map(|(key, version_id)| {
                (
                    key.to_string(),
                    FakeObject::new(1).with_version_id(version_id),
                )
            })
            .collect(),
        versioning_enabled: true,
        versions_xml: Some(versions_xml),
    }
}

fn versioned_sync_args(fake: &FakeS3, extra_args: &[&str]) -> Vec<String> {
    let mut args = strings(&["--enable-versioning", "--disable-etag-verify"]);
    args.extend(strings(extra_args));
    args.extend(client_args("source", &fake.endpoint));
    args.extend(client_args("target", &fake.endpoint));
    args.push("s3://source-bucket/".to_string());
    args.push("s3://target-bucket/".to_string());
    args
}

/// With `--enable-versioning`, the origin version ID is what tells the next
/// run that a version is already synced. `--metadata` used to drop it, so
/// the same versions were synced again on every run.
#[test]
fn metadata_option_keeps_origin_version_id_with_versioning() {
    let fixture = Fixture::default()
        .bucket(
            "source-bucket",
            versioned_bucket(
                versions_page(&version_entry("key.txt", "version1", true)),
                &[("key.txt", "version1")],
            ),
        )
        .bucket("target-bucket", versioned_bucket(String::new(), &[]));
    let fake = FakeS3::serve(fixture);

    let (code, stdout, stderr) =
        run_sync(versioned_sync_args(&fake, &["--metadata", "key1=value1"]));
    assert_eq!(code, Some(0), "stdout={stdout}\nstderr={stderr}");

    let puts = fake.requests_of("PUT", Operation::PutObject);
    assert_eq!(puts.len(), 1, "requests:\n{}", fake.request_log());
    let put = &puts[0];
    assert_eq!(put.path(), "/target-bucket/key.txt");
    assert_eq!(put.header("x-amz-meta-key1"), Some("value1"));
    assert_eq!(
        put.header("x-amz-meta-s3sync_origin_version_id"),
        Some("version1"),
        "headers: {:?}",
        put.headers
    );
    assert!(
        put.header("x-amz-meta-s3sync_origin_last_modified")
            .is_some(),
        "headers: {:?}",
        put.headers
    );
}

// ---------------------------------------------------------------
// Force retry for multipart upload requests and CopyObject
// ---------------------------------------------------------------

/// Upload one two-part file to a fake endpoint that drops the connection
/// for the first `operation` request, and return the endpoint for request
/// assertions. The sync must recover through force retry: exit 3 (warning),
/// with the upload completed.
fn two_part_upload_with_one_dropped(operation: Operation) -> FakeS3 {
    let fixture = Fixture::default();
    let fail = FailFirst::new(1);
    let fake = FakeS3::spawn(move |request| {
        if operation.matches(request) && fail.should_fail() {
            return Reply::CloseConnection;
        }
        fixture.respond(request)
    });
    let source_dir = two_part_source_dir();

    let mut args = strings(&TWO_PART_UPLOAD_ARGS);
    args.extend(client_args("target", &fake.endpoint));
    args.push(dir_arg(source_dir.path()));
    args.push("s3://fake-bucket/".to_string());
    let (code, stdout, stderr) = run_sync(args);

    assert_eq!(
        code,
        Some(3),
        "a force-retried {operation:?} must end as a warning; stdout={stdout}\nstderr={stderr}\nrequests:\n{}",
        fake.request_log()
    );
    assert!(
        stdout.contains("force retryable error has occurred."),
        "stdout={stdout}"
    );
    fake
}

#[test]
fn force_retry_applies_to_create_multipart_upload_connection_error() {
    let fake = two_part_upload_with_one_dropped(Operation::CreateMultipartUpload);

    assert_eq!(fake.count("POST", Operation::CreateMultipartUpload), 2);
    assert_eq!(fake.count("PUT", Operation::UploadPart), 2);
    assert_eq!(fake.count("POST", Operation::CompleteMultipartUpload), 1);
}

#[test]
fn force_retry_applies_to_upload_part_connection_error() {
    let fake = two_part_upload_with_one_dropped(Operation::UploadPart);

    // The failed attempt is aborted, then the whole upload is started again.
    assert_eq!(fake.count("DELETE", Operation::AbortMultipartUpload), 1);
    assert_eq!(fake.count("POST", Operation::CreateMultipartUpload), 2);
    assert_eq!(fake.count("POST", Operation::CompleteMultipartUpload), 1);
}

#[test]
fn force_retry_applies_to_complete_multipart_upload_connection_error() {
    let fake = two_part_upload_with_one_dropped(Operation::CompleteMultipartUpload);

    assert_eq!(fake.count("POST", Operation::CreateMultipartUpload), 2);
    assert_eq!(fake.count("POST", Operation::CompleteMultipartUpload), 2);
}

/// A failed `AbortMultipartUpload` must not replace the upload error: the
/// upload error is the one that decides whether the object is retried.
#[test]
fn force_retry_survives_a_failed_abort_multipart_upload() {
    let fixture = Fixture::default();
    let fail_part = FailFirst::new(1);
    let fake = FakeS3::spawn(move |request| {
        if Operation::UploadPart.matches(request) && fail_part.should_fail() {
            return Reply::CloseConnection;
        }
        if Operation::AbortMultipartUpload.matches(request) {
            return Reply::error(500, "InternalError");
        }
        fixture.respond(request)
    });
    let source_dir = two_part_source_dir();

    let mut args = strings(&TWO_PART_UPLOAD_ARGS);
    args.extend(client_args("target", &fake.endpoint));
    args.push(dir_arg(source_dir.path()));
    args.push("s3://fake-bucket/".to_string());
    let (code, stdout, stderr) = run_sync(args);

    assert_eq!(
        code,
        Some(3),
        "stdout={stdout}\nstderr={stderr}\nrequests:\n{}",
        fake.request_log()
    );
    assert!(
        stdout.contains("failed to abort the multipart upload"),
        "the abort failure must be logged as a warning; stdout={stdout}"
    );
    assert_eq!(fake.count("DELETE", Operation::AbortMultipartUpload), 1);
    assert_eq!(fake.count("POST", Operation::CreateMultipartUpload), 2);
    assert_eq!(fake.count("POST", Operation::CompleteMultipartUpload), 1);
}

fn server_side_copy_args(fake: &FakeS3, extra_args: &[&str]) -> Vec<String> {
    let mut args = strings(&[
        "--server-side-copy",
        "--disable-etag-verify",
        "--force-retry-count",
        "2",
        "--force-retry-interval-milliseconds",
        "1",
    ]);
    args.extend(strings(extra_args));
    args.extend(client_args("source", &fake.endpoint));
    args.extend(client_args("target", &fake.endpoint));
    args.push("s3://source-bucket/".to_string());
    args.push("s3://target-bucket/".to_string());
    args
}

fn bucket_with(objects: &[(&str, FakeObject)]) -> FakeBucket {
    FakeBucket {
        objects: objects
            .iter()
            .map(|(key, object)| (key.to_string(), object.clone()))
            .collect(),
        ..FakeBucket::default()
    }
}

#[test]
fn force_retry_applies_to_copy_object_connection_error() {
    let fixture = Fixture::default().bucket(
        "source-bucket",
        bucket_with(&[("key.txt", FakeObject::new(3))]),
    );
    let fail = FailFirst::new(1);
    let fake = FakeS3::spawn(move |request| {
        if Operation::CopyObject.matches(request) && fail.should_fail() {
            return Reply::CloseConnection;
        }
        fixture.respond(request)
    });

    let (code, stdout, stderr) = run_sync(server_side_copy_args(&fake, &[]));

    assert_eq!(
        code,
        Some(3),
        "stdout={stdout}\nstderr={stderr}\nrequests:\n{}",
        fake.request_log()
    );
    assert!(
        stdout.contains("force retryable error has occurred."),
        "stdout={stdout}"
    );
    assert_eq!(fake.count("PUT", Operation::CopyObject), 2);
}

#[test]
fn force_retry_applies_to_upload_part_copy_connection_error() {
    let fixture = Fixture::default().bucket(
        "source-bucket",
        bucket_with(&[("big.dat", FakeObject::new(5 * MIB + 1024))]),
    );
    let fail = FailFirst::new(1);
    let fake = FakeS3::spawn(move |request| {
        if Operation::UploadPartCopy.matches(request) && fail.should_fail() {
            return Reply::CloseConnection;
        }
        fixture.respond(request)
    });

    let (code, stdout, stderr) = run_sync(server_side_copy_args(
        &fake,
        &[
            "--multipart-threshold",
            "5MiB",
            "--multipart-chunksize",
            "5MiB",
        ],
    ));

    assert_eq!(
        code,
        Some(3),
        "stdout={stdout}\nstderr={stderr}\nrequests:\n{}",
        fake.request_log()
    );
    assert!(
        stdout.contains("force retryable error has occurred."),
        "stdout={stdout}"
    );
    assert_eq!(fake.count("POST", Operation::CreateMultipartUpload), 2);
    assert_eq!(fake.count("POST", Operation::CompleteMultipartUpload), 1);
}

/// An error S3 answered with is not a connection error, so it must not be
/// force-retried: a failed precondition is reported once, as a warning.
#[test]
fn copy_object_precondition_failure_is_not_force_retried() {
    let fixture = Fixture::default().bucket(
        "source-bucket",
        bucket_with(&[("key.txt", FakeObject::new(3))]),
    );
    let fake = FakeS3::spawn(move |request| {
        if Operation::CopyObject.matches(request) {
            return Reply::error(412, "PreconditionFailed");
        }
        fixture.respond(request)
    });

    let (code, stdout, stderr) = run_sync(server_side_copy_args(&fake, &[]));

    assert_eq!(
        code,
        Some(3),
        "stdout={stdout}\nstderr={stderr}\nrequests:\n{}",
        fake.request_log()
    );
    assert!(
        !stdout.contains("force retryable error has occurred."),
        "stdout={stdout}"
    );
    assert_eq!(fake.count("PUT", Operation::CopyObject), 1);
}

// ---------------------------------------------------------------
// --if-match with --server-side-copy
// ---------------------------------------------------------------

/// `--if-match` was not added to single-part `CopyObject` requests, so the
/// target object was overwritten unconditionally.
#[test]
fn if_match_is_sent_for_single_part_server_side_copy() {
    const TARGET_E_TAG: &str = "\"11111111111111111111111111111111\"";
    let fixture = Fixture::default()
        .bucket(
            "source-bucket",
            bucket_with(&[("key.txt", FakeObject::new(3))]),
        )
        .bucket(
            "target-bucket",
            // Older than the source object, so the object is synced.
            bucket_with(&[(
                "key.txt",
                FakeObject::new(5).with_e_tag(TARGET_E_TAG).older(),
            )]),
        );
    let fake = FakeS3::serve(fixture);

    let (code, stdout, stderr) = run_sync(server_side_copy_args(&fake, &["--if-match"]));
    assert_eq!(code, Some(0), "stdout={stdout}\nstderr={stderr}");

    let copies = fake.requests_of("PUT", Operation::CopyObject);
    assert_eq!(copies.len(), 1, "requests:\n{}", fake.request_log());
    assert_eq!(copies[0].path(), "/target-bucket/key.txt");
    assert_eq!(
        copies[0].header("if-match"),
        Some(TARGET_E_TAG),
        "CopyObject must carry the target ETag; headers: {:?}",
        copies[0].headers
    );
}

/// Without `--if-match`, no `If-Match` header is sent.
#[test]
fn if_match_is_not_sent_for_server_side_copy_by_default() {
    let fixture = Fixture::default()
        .bucket(
            "source-bucket",
            bucket_with(&[("key.txt", FakeObject::new(3))]),
        )
        .bucket(
            "target-bucket",
            bucket_with(&[("key.txt", FakeObject::new(5).older())]),
        );
    let fake = FakeS3::serve(fixture);

    let (code, stdout, stderr) = run_sync(server_side_copy_args(&fake, &[]));
    assert_eq!(code, Some(0), "stdout={stdout}\nstderr={stderr}");

    let copies = fake.requests_of("PUT", Operation::CopyObject);
    assert_eq!(copies.len(), 1, "requests:\n{}", fake.request_log());
    assert!(copies[0].header("if-match").is_none());
}

// ---------------------------------------------------------------
// Multipart download: a failed ranged GetObject is an error
// ---------------------------------------------------------------

/// The ranged `GetObject` for the second part fails. This used to panic in
/// the part task; it must fail the sync like any other download error and
/// leave no file behind.
#[test]
fn failed_ranged_get_object_for_a_later_part_is_an_error() {
    let fixture = Fixture::default().bucket(
        "source-bucket",
        bucket_with(&[("big.dat", FakeObject::new(5 * MIB + 1024))]),
    );
    let fake = FakeS3::spawn(move |request| {
        let is_later_part = request
            .header("range")
            .is_some_and(|range| !range.starts_with("bytes=0-"));
        if Operation::GetObject.matches(request) && is_later_part {
            return Reply::error(500, "InternalError");
        }
        fixture.respond(request)
    });
    let target_dir = tempfile::tempdir().unwrap();

    let mut args = strings(&[
        "--multipart-threshold",
        "5MiB",
        "--multipart-chunksize",
        "5MiB",
        "--disable-etag-verify",
        "--force-retry-count",
        "0",
    ]);
    args.extend(client_args("source", &fake.endpoint));
    args.push("s3://source-bucket/".to_string());
    args.push(dir_arg(target_dir.path()));
    let (code, stdout, stderr) = run_sync(args);

    assert_no_panic(&stdout, &stderr);
    assert_eq!(
        code,
        Some(1),
        "stdout={stdout}\nstderr={stderr}\nrequests:\n{}",
        fake.request_log()
    );
    assert!(
        stdout.contains("get_object() failed."),
        "the download error must be reported; stdout={stdout}"
    );

    let ranges: Vec<String> = fake
        .requests_of("GET", Operation::GetObject)
        .iter()
        .filter_map(|request| request.header("range").map(str::to_string))
        .collect();
    assert!(
        ranges.iter().any(|range| range.starts_with("bytes=0-"))
            && ranges.iter().any(|range| !range.starts_with("bytes=0-")),
        "both parts must have been requested; ranges: {ranges:?}"
    );
    assert!(!target_dir.path().join("big.dat").exists());
}

// ---------------------------------------------------------------
// --check-etag --auto-chunksize for a single-part object
// ---------------------------------------------------------------

fn check_e_tag_auto_chunksize(target_e_tag: &str) -> (FakeS3, Option<i32>, String, String) {
    // Not smaller than the default multipart threshold (8MiB).
    const SIZE: usize = 8 * MIB;
    let source_dir = tempfile::tempdir().unwrap();
    std::fs::write(source_dir.path().join("big.dat"), vec![b'x'; SIZE]).unwrap();

    let fixture = Fixture::default().bucket(
        "target-bucket",
        bucket_with(&[("big.dat", FakeObject::new(SIZE).with_e_tag(target_e_tag))]),
    );
    let fake = FakeS3::serve(fixture);

    let mut args = strings(&["--check-etag", "--auto-chunksize", "--disable-etag-verify"]);
    args.extend(client_args("target", &fake.endpoint));
    args.push(dir_arg(source_dir.path()));
    args.push("s3://target-bucket/".to_string());
    let (code, stdout, stderr) = run_sync(args);
    (fake, code, stdout, stderr)
}

fn count_writes(fake: &FakeS3) -> usize {
    fake.count("PUT", Operation::PutObject) + fake.count("POST", Operation::CreateMultipartUpload)
}

/// The target object was uploaded in a single part, so its ETag is the MD5
/// digest of the whole object even though the local file is not smaller
/// than the multipart threshold. The file used to be hashed as a multipart
/// upload, which never matched, so the object was uploaded on every run.
#[test]
fn check_etag_auto_chunksize_matches_single_part_object_over_threshold() {
    let whole_file_e_tag = format!("\"{:x}\"", md5::compute(vec![b'x'; 8 * MIB]));
    let (fake, code, stdout, stderr) = check_e_tag_auto_chunksize(&whole_file_e_tag);

    assert_eq!(code, Some(0), "stdout={stdout}\nstderr={stderr}");
    assert_eq!(
        count_writes(&fake),
        0,
        "an unmodified object must not be uploaded; requests:\n{}",
        fake.request_log()
    );
}

/// Control for the test above: a different single-part ETag is still a
/// modification, so the object is uploaded.
#[test]
fn check_etag_auto_chunksize_uploads_modified_single_part_object() {
    let (fake, code, stdout, stderr) =
        check_e_tag_auto_chunksize("\"11111111111111111111111111111111\"");

    assert_eq!(code, Some(0), "stdout={stdout}\nstderr={stderr}");
    assert_eq!(
        count_writes(&fake),
        1,
        "a modified object must be uploaded; requests:\n{}",
        fake.request_log()
    );
}

// ---------------------------------------------------------------
// --enable-versioning: delete markers and sync warnings
// ---------------------------------------------------------------

/// A delete marker reaches the Lua filter callback before the versions are
/// packed. It has no checksum, so the checksum fields must be `nil`;
/// reading them used to panic.
#[test]
fn lua_filter_callback_receives_delete_marker_with_nil_checksum() {
    let versions = versions_page(&format!(
        "{}{}{}",
        delete_marker_entry("gone.txt", "marker1"),
        version_entry("gone.txt", "version0", false),
        version_entry("key.txt", "version1", true),
    ));
    let fixture = Fixture::default()
        .bucket(
            "source-bucket",
            versioned_bucket(
                versions,
                &[("key.txt", "version1"), ("gone.txt", "version0")],
            ),
        )
        .bucket("target-bucket", versioned_bucket(String::new(), &[]));
    let fake = FakeS3::serve(fixture);

    let script_dir = tempfile::tempdir().unwrap();
    let script = script_dir.path().join("filter.lua");
    std::fs::write(
        &script,
        r#"function filter(source_object)
    print("FILTER key=" .. source_object.key
        .. " is_delete_marker=" .. tostring(source_object.is_delete_marker)
        .. " checksum_algorithm=" .. tostring(source_object.checksum_algorithm)
        .. " checksum_type=" .. tostring(source_object.checksum_type))
    return true
end
"#,
    )
    .unwrap();

    let (code, stdout, stderr) = run_sync(versioned_sync_args(
        &fake,
        &["--filter-callback-lua-script", script.to_str().unwrap()],
    ));

    assert_no_panic(&stdout, &stderr);
    assert_eq!(
        code,
        Some(0),
        "stdout={stdout}\nstderr={stderr}\nrequests:\n{}",
        fake.request_log()
    );
    assert!(
        stdout.contains(
            "FILTER key=gone.txt is_delete_marker=true checksum_algorithm=nil checksum_type=nil"
        ),
        "the delete marker must reach the filter with nil checksum fields; stdout={stdout}"
    );
    assert!(
        stdout.contains("FILTER key=key.txt is_delete_marker=false"),
        "stdout={stdout}"
    );

    // Both versions are copied, and the delete marker is replayed.
    assert_eq!(fake.count("PUT", Operation::PutObject), 2);
    let deletes: Vec<Request> = fake
        .requests()
        .into_iter()
        .filter(|request| request.method == "DELETE")
        .collect();
    assert_eq!(deletes.len(), 1, "requests:\n{}", fake.request_log());
    assert_eq!(deletes[0].path(), "/target-bucket/gone.txt");
}

/// The versions listed for the source no longer exist when they are
/// fetched, which is a sync warning ("object not found"). Building the
/// warning event used to panic for an object holding several versions.
#[test]
fn sync_warning_with_enable_versioning_does_not_panic() {
    let versions = versions_page(&format!(
        "{}{}",
        version_entry("key.txt", "version2", true),
        version_entry("key.txt", "version1", false),
    ));
    let fixture = Fixture::default()
        // Listed, but no object can be fetched.
        .bucket("source-bucket", versioned_bucket(versions, &[]))
        .bucket("target-bucket", versioned_bucket(String::new(), &[]));
    let fake = FakeS3::serve(fixture);

    let (code, stdout, stderr) = run_sync(versioned_sync_args(&fake, &[]));

    assert_no_panic(&stdout, &stderr);
    assert_eq!(
        code,
        Some(3),
        "a sync warning must exit 3; stdout={stdout}\nstderr={stderr}\nrequests:\n{}",
        fake.request_log()
    );
    assert!(
        stdout.contains("object not found. skipping."),
        "stdout={stdout}"
    );
    assert_eq!(fake.count("PUT", Operation::PutObject), 0);
}
