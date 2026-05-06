// Vendored from s3util-rs@1.3.0
//   src/bin/s3util/cli/put_bucket_request_payment.rs
// Adjustments: no tests stripped; rewrote crate::cli → super
use anyhow::Result;
use aws_sdk_s3::types::RequestPaymentConfiguration;
use tracing::info;

use s3util_rs::config::ClientConfig;
use s3util_rs::config::args::put_bucket_request_payment::PutBucketRequestPaymentArgs;
use s3util_rs::storage::s3::api;

/// Runtime entry for
/// `s3util put-bucket-request-payment s3://<BUCKET> --requester|--bucket-owner`.
///
/// Builds the SDK client from `client_config`, issues `PutBucketRequestPayment`
/// with `Payer=Requester` or `Payer=BucketOwner` (determined by the
/// mutually-exclusive `--requester` / `--bucket-owner` flags), and exits
/// silently on success.
pub async fn run_put_bucket_request_payment(
    args: PutBucketRequestPaymentArgs,
    client_config: ClientConfig,
) -> Result<()> {
    args.validate_state_flag();

    let bucket = args
        .bucket_name()
        .map_err(|e| anyhow::anyhow!("{}", e.trim_end()))?;
    let payer = args.payer();
    let cfg = RequestPaymentConfiguration::builder()
        .payer(payer.clone())
        .build()?;
    let client = client_config.create_client().await;
    if args.dry_run {
        info!(
            bucket = %bucket,
            payer = %payer.as_str(),
            "[dry-run] would put bucket request payment."
        );
        return Ok(());
    }
    api::put_bucket_request_payment(&client, &bucket, cfg).await?;
    info!(
        bucket = %bucket,
        payer = %payer.as_str(),
        "Bucket request payment set."
    );
    Ok(())
}
