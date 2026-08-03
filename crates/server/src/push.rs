//! Telling the devices that a Question Set has arrived.
//!
//! One push per subscribed device, encrypted for that device's own keys and
//! signed with the VAPID identity the store generated on first run. The body is
//! small on purpose: enough for the service worker to draw the notification and
//! to know which Set to open, and nothing that would put a Question in a
//! notification the phone shows on a lock screen.
//!
//! Sending happens behind the answer to `POST /api/v1/sets`, never in front of
//! it. Delivery goes out through the browser vendors' push services, which is
//! the one place Askance reaches the public internet, and none of it is
//! reliable enough to make an agent's submission depend on: a service that
//! cannot be reached costs a notification, not the Set.
//!
//! A push service is also the only thing that can tell us a device has gone —
//! the app was uninstalled, the subscription expired — and it says so with a
//! `404` or a `410`. Those are pruned. Everything else, a timeout or a `503`,
//! is a notification lost and the device is left alone.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use askance_schema::QuestionSet;
use askance_store::{PushSubscription, VapidKeys};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use p256::ecdsa::signature::Signer;
use serde::Serialize;
use sqlx::SqlitePool;
use time::OffsetDateTime;
use web_push_native::{Auth, WebPushBuilder};

/// How long a push service should hold a notification for a phone that is off
/// or out of signal. Half a day: a Question Set older than that has either been
/// answered at a desk or is not going to be answered from a lock screen.
const HOLD_FOR: Duration = Duration::from_secs(12 * 60 * 60);

/// How long to wait on one push service before giving up on it. A push service
/// that is this slow has cost this device its notification either way, and the
/// remaining devices are still waiting their turn.
const REACHABLE_WITHIN: Duration = Duration::from_secs(10);

/// How long a VAPID signature stays good for. RFC 8292 caps it at 24 hours;
/// each push is signed as it is sent, so this only has to outlive the request.
const SIGNATURE_GOOD_FOR: i64 = 12 * 60 * 60;

/// The contact the VAPID claim carries: whom a push service would complain to
/// about this server's traffic. It wants a `mailto:` or an `https:` URI, and
/// the project is the only thing that is true of every Askance — this is a
/// single-user tool with nobody's address configured in it.
const CONTACT: &str = "https://github.com/tobico/askance";

/// The one JWT header these signatures ever use, so it is spelled out rather
/// than serialized: ES256 is what VAPID is defined in terms of.
const JWT_HEADER: &str = r#"{"typ":"JWT","alg":"ES256"}"#;

/// What the service worker is handed: enough to draw the notification, and the
/// id to open.
#[derive(Debug, Serialize)]
struct Notice<'a> {
    id: i64,
    title: &'a str,

    /// Which repository the agent is asking about, when the CLI knew — the one
    /// thing that tells two Sets apart at a glance on a lock screen.
    #[serde(skip_serializing_if = "Option::is_none")]
    project: Option<&'a str>,
}

/// What became of one push.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Delivery {
    /// The push service took it. Whether the phone ever shows it is between the
    /// two of them.
    Taken,

    /// This subscription is finished with, for good. The device uninstalled the
    /// app, or its subscription expired.
    Gone,
}

/// Tell every subscribed device about a Set, without making the agent wait for
/// it.
///
/// Returns as soon as the work is handed to the runtime: the caller's job is to
/// answer the agent, and this must not be able to delay that or to fail it.
pub(crate) fn announce(pool: &SqlitePool, id: i64, set: &QuestionSet) {
    let notice = Notice {
        id,
        title: &set.title,
        project: set.project.as_deref(),
    };

    let notice = match serde_json::to_vec(&notice) {
        Ok(notice) => notice,
        Err(error) => {
            tracing::error!(set = id, error = ?error, "the push notice could not be built");
            return;
        }
    };

    let pool = pool.clone();
    tokio::spawn(async move {
        if let Err(error) = notify(&pool, id, &notice).await {
            tracing::error!(set = id, error = ?error, "telling the devices about a Set failed");
        }
    });
}

/// Send the notice to every device, and prune the ones the push services have
/// finished with.
///
/// One device at a time rather than all at once: this is a single human's
/// handful of devices, and the timeout bounds what a slow push service can cost
/// the ones behind it.
async fn notify(pool: &SqlitePool, id: i64, notice: &[u8]) -> Result<()> {
    let devices = askance_store::push_subscriptions(pool).await?;
    if devices.is_empty() {
        return Ok(());
    }

    let keys = askance_store::vapid_keys(pool).await?;
    let client = reqwest::Client::builder()
        .timeout(REACHABLE_WITHIN)
        .build()
        .context("building the push client")?;

    for device in devices {
        match send(&client, &keys, &device, notice).await {
            Ok(Delivery::Taken) => {
                tracing::debug!(set = id, endpoint = %device.endpoint, "a device was told");
            }
            Ok(Delivery::Gone) => {
                tracing::info!(
                    endpoint = %device.endpoint,
                    "the push service has finished with this device, so it is forgotten",
                );
                askance_store::forget_subscription(pool, &device.endpoint).await?;
            }
            // Logged rather than returned: the devices behind this one have
            // done nothing wrong, and there is nobody to hand the failure to —
            // the agent was answered before any of this started.
            Err(error) => {
                tracing::warn!(
                    set = id,
                    endpoint = %device.endpoint,
                    error = ?error,
                    "a device was not told about a Set",
                );
            }
        }
    }

    Ok(())
}

/// Put one push on the wire, and read what the push service made of it.
async fn send(
    client: &reqwest::Client,
    keys: &VapidKeys,
    device: &PushSubscription,
    notice: &[u8],
) -> Result<Delivery> {
    let request = addressed(keys, device, notice)?;

    let response = client
        .execute(reqwest::Request::try_from(request).context("preparing the push request")?)
        .await
        .context("reaching the push service")?;

    let status = response.status();

    if status.is_success() {
        return Ok(Delivery::Taken);
    }

    // The only two answers that mean the subscription itself is over. Anything
    // else — a timeout, a 429, a 503 — is this notification lost and no word at
    // all about the device.
    if status == reqwest::StatusCode::NOT_FOUND || status == reqwest::StatusCode::GONE {
        return Ok(Delivery::Gone);
    }

    let said = response.text().await.unwrap_or_default();
    bail!("the push service answered {status}: {said}");
}

/// The notice encrypted for one device, in the request its push service takes.
fn addressed(
    keys: &VapidKeys,
    device: &PushSubscription,
    notice: &[u8],
) -> Result<http::Request<Vec<u8>>> {
    let endpoint: http::Uri = device
        .endpoint
        .parse()
        .with_context(|| format!("the stored endpoint {:?} is not a URL", device.endpoint))?;

    let public = URL_SAFE_NO_PAD
        .decode(&device.p256dh)
        .context("the stored p256dh is not base64url")?;
    let public = p256::PublicKey::from_sec1_bytes(&public)
        .context("the stored p256dh is not a P-256 key")?;

    let auth = URL_SAFE_NO_PAD
        .decode(&device.auth)
        .context("the stored auth secret is not base64url")?;
    if auth.len() != 16 {
        bail!("the stored auth secret is {} bytes, not 16", auth.len());
    }

    let mut request = WebPushBuilder::new(endpoint.clone(), public, Auth::clone_from_slice(&auth))
        .with_valid_duration(HOLD_FOR)
        .build(notice.to_vec())
        .map_err(|err| anyhow::anyhow!("encrypting the push: {err}"))?;

    let authorization = authorization(keys, &endpoint)?;
    request.headers_mut().insert(
        http::header::AUTHORIZATION,
        authorization
            .parse()
            .context("the VAPID header is not a header value")?,
    );

    Ok(request)
}

/// The `Authorization` header a push service checks this server by: a signed
/// claim about who is sending, alongside the public key it was signed with —
/// which is the key the device subscribed against.
fn authorization(keys: &VapidKeys, endpoint: &http::Uri) -> Result<String> {
    let claims = serde_json::json!({
        "aud": audience(endpoint)?,
        "exp": OffsetDateTime::now_utc().unix_timestamp() + SIGNATURE_GOOD_FOR,
        "sub": CONTACT,
    });

    let token = signed(
        &keys.private_key,
        &serde_json::to_vec(&claims).context("serialising the VAPID claims")?,
    )?;

    Ok(format!("vapid t={token}, k={}", keys.public_key))
}

/// Who a signature is for: the push service's origin, and nothing of the path
/// that identifies the device. A signature made out to one service is no use at
/// another, which is the whole point of the claim.
fn audience(endpoint: &http::Uri) -> Result<String> {
    let scheme = endpoint
        .scheme_str()
        .context("the endpoint has no scheme to make the signature out to")?;
    let authority = endpoint
        .authority()
        .context("the endpoint has no host to make the signature out to")?;

    Ok(format!("{scheme}://{authority}"))
}

/// The claims as a signed ES256 JWT.
///
/// Signed with `p256` directly rather than through a JWT crate: this is one
/// signature over two claims under a fixed header, and the keypair is already
/// a `p256` one.
fn signed(private_key: &str, claims: &[u8]) -> Result<String> {
    let scalar = URL_SAFE_NO_PAD
        .decode(private_key)
        .context("the stored private key is not base64url")?;
    let key = p256::ecdsa::SigningKey::from_slice(&scalar)
        .context("the stored private key is not a P-256 scalar")?;

    let signing_input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(JWT_HEADER),
        URL_SAFE_NO_PAD.encode(claims),
    );

    // Fixed-width r‖s rather than DER: JWS defines ES256 as the 64-byte form,
    // and a push service handed DER rejects the signature.
    let signature: p256::ecdsa::Signature = key.sign(signing_input.as_bytes());

    Ok(format!(
        "{signing_input}.{}",
        URL_SAFE_NO_PAD.encode(signature.to_bytes()),
    ))
}

#[cfg(test)]
mod tests {
    use super::{JWT_HEADER, audience, signed};
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use p256::ecdsa::signature::Verifier;
    use p256::elliptic_curve::rand_core::OsRng;
    use p256::elliptic_curve::sec1::ToEncodedPoint;

    #[test]
    fn a_signature_is_one_a_push_service_can_check_against_the_public_key() {
        let secret = p256::SecretKey::random(&mut OsRng);
        let private_key = URL_SAFE_NO_PAD.encode(secret.to_bytes());

        let token = signed(&private_key, br#"{"aud":"https://push.example"}"#).unwrap();

        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3, "a JWT is three parts: {token}");
        assert_eq!(
            URL_SAFE_NO_PAD.decode(parts[0]).unwrap(),
            JWT_HEADER.as_bytes(),
        );

        let signature =
            p256::ecdsa::Signature::from_slice(&URL_SAFE_NO_PAD.decode(parts[2]).unwrap())
                .expect("the signature has to be the 64-byte r‖s form JWS defines");

        // Through the encoding the browser was handed, because that is the key
        // the push service checks against: the `k=` half of the header.
        let handed_out = secret.public_key().to_encoded_point(false);
        let verifying = p256::ecdsa::VerifyingKey::from_sec1_bytes(handed_out.as_bytes()).unwrap();

        verifying
            .verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &signature)
            .expect("the push service has to be able to check the signature");
    }

    #[test]
    fn a_signature_is_made_out_to_the_push_service_and_not_to_the_device() {
        let endpoint = "https://push.example/devices/abc123?token=xyz"
            .parse()
            .unwrap();

        assert_eq!(audience(&endpoint).unwrap(), "https://push.example");
    }

    #[test]
    fn a_push_service_on_an_unusual_port_is_a_different_audience() {
        let endpoint = "http://127.0.0.1:8422/devices/abc123".parse().unwrap();

        assert_eq!(audience(&endpoint).unwrap(), "http://127.0.0.1:8422");
    }
}
