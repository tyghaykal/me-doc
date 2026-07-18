use rand::Rng;
use redis::AsyncCommands;

use super::error::AuthError;
use super::util::sha256_b64;

const MAX_ATTEMPTS: u32 = 5;
const COOLDOWN_SECONDS: i64 = 60;

fn code_key(purpose: &str, email: &str) -> String {
    format!("otp:code:{purpose}:{email}")
}

fn attempts_key(purpose: &str, email: &str) -> String {
    format!("otp:attempts:{purpose}:{email}")
}

fn cooldown_key(purpose: &str, email: &str) -> String {
    format!("otp:cooldown:{purpose}:{email}")
}

/// Generates and stores a new OTP for (purpose, email), returning the plaintext code to email to the user.
pub async fn issue_otp(
    redis: &redis::Client,
    purpose: &str,
    email: &str,
    ttl_seconds: i64,
) -> Result<String, AuthError> {
    let mut conn = redis.get_multiplexed_async_connection().await?;

    let on_cooldown: bool = conn.exists(cooldown_key(purpose, email)).await?;
    if on_cooldown {
        return Err(AuthError::OtpCooldown);
    }

    let code: u32 = rand::thread_rng().gen_range(0..1_000_000);
    let code = format!("{code:06}");

    let () = conn
        .set_ex(code_key(purpose, email), sha256_b64(&code), ttl_seconds as u64)
        .await?;
    let _: i64 = conn.del(attempts_key(purpose, email)).await?;
    let () = conn
        .set_ex(cooldown_key(purpose, email), 1, COOLDOWN_SECONDS as u64)
        .await?;

    Ok(code)
}

/// Verifies a submitted OTP code, consuming it on success.
pub async fn verify_otp(
    redis: &redis::Client,
    purpose: &str,
    email: &str,
    code: &str,
) -> Result<(), AuthError> {
    let mut conn = redis.get_multiplexed_async_connection().await?;

    let stored: Option<String> = conn.get(code_key(purpose, email)).await?;
    let Some(stored) = stored else {
        return Err(AuthError::InvalidOtp);
    };

    let attempts: u32 = conn.incr(attempts_key(purpose, email), 1).await?;
    if attempts == 1 {
        let _: () = conn.expire(attempts_key(purpose, email), 900).await?;
    }
    if attempts > MAX_ATTEMPTS {
        let _: i64 = conn.del(code_key(purpose, email)).await?;
        return Err(AuthError::OtpLocked);
    }

    if sha256_b64(code) != stored {
        return Err(AuthError::InvalidOtp);
    }

    let _: i64 = conn.del(code_key(purpose, email)).await?;
    let _: i64 = conn.del(attempts_key(purpose, email)).await?;
    Ok(())
}
