use lettre::message::MultiPart;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

/// Escapes the characters that matter inside an HTML text node/attribute.
/// `page_title`/`inviter_email` are user-controlled and interpolated
/// directly into the email templates below via `format!`, which does no
/// escaping on its own.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[derive(Clone)]
pub struct EmailClient {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
    product_name: String,
    /// Browser-reachable app origin (config.frontend_origin), used for the
    /// "Open the app" CTA in welcome emails.
    app_url: String,
}

impl EmailClient {
    pub fn new(host: &str, port: u16, from: &str, product_name: &str, app_url: &str) -> anyhow::Result<Self> {
        let transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
            .port(port)
            .build();
        Ok(Self {
            transport,
            from: from.to_string(),
            product_name: product_name.to_string(),
            app_url: app_url.trim_end_matches('/').to_string(),
        })
    }

    pub async fn send_otp(&self, to: &str, purpose: &str, code: &str) -> anyhow::Result<()> {
        let action = match purpose {
            "register" => "verify your email",
            "login" => "complete your login",
            _ => "continue",
        };
        let subject = format!("Your {} verification code", self.product_name);

        let plain = format!(
            "Use this code to {action}: {code}\n\nThis code expires shortly and can only be used once.\nIf you didn't request this, you can ignore this email."
        );
        let html = otp_html(action, code, &self.product_name);

        let email = Message::builder()
            .from(self.from.parse()?)
            .to(to.parse()?)
            .subject(subject)
            .multipart(MultiPart::alternative_plain_html(plain, html))?;

        self.transport.send(email).await?;
        Ok(())
    }

    /// Registration OTP email that ALSO carries the welcome message, so a
    /// password-registered user gets their welcome in the same email they're
    /// already reading to verify their account. `is_new_user` is always true
    /// here (register only ever creates a new account); it's kept to share the
    /// copy with the standalone welcome email.
    pub async fn send_register_otp_with_welcome(
        &self,
        to: &str,
        code: &str,
    ) -> anyhow::Result<()> {
        let name = &self.product_name;
        let subject = format!("Welcome to {name} — verify your email");

        let plain = format!(
            "Welcome to {name}!\n\nUse this code to verify your email: {code}\n\nThis code expires shortly and can only be used once.\nIf you didn't request this, you can ignore this email."
        );
        let html = register_otp_welcome_html(code, name);

        let email = Message::builder()
            .from(self.from.parse()?)
            .to(to.parse()?)
            .subject(subject)
            .multipart(MultiPart::alternative_plain_html(plain, html))?;

        self.transport.send(email).await?;
        Ok(())
    }

    /// Welcome email sent when an account is created or signed into via Google
    /// OAuth (no OTP involved — Google already proved ownership of the email).
    /// `is_new_user` tweaks the CTA: a fresh account is told to open the app
    /// and get started; a returning user is just greeted and linked in.
    pub async fn send_welcome(&self, to: &str, is_new_user: bool) -> anyhow::Result<()> {
        let name = &self.product_name;
        let app_url = &self.app_url;
        let subject = if is_new_user {
            format!("Welcome to {name}!")
        } else {
            format!("Welcome back to {name}")
        };

        let (plain, html) = if is_new_user {
            (
                format!(
                    "Welcome to {name}!\n\nYour account is ready. Open the app to create your first document and start writing together.\n\n{app_url}"
                ),
                welcome_html(name, "Open the app", app_url),
            )
        } else {
            (
                format!(
                    "Welcome back to {name}!\n\nYour Google account is now linked to {name}. You can keep using the same documents and workspaces as before.\n\n{app_url}"
                ),
                welcome_html(name, "Open the app", app_url),
            )
        };

        let email = Message::builder()
            .from(self.from.parse()?)
            .to(to.parse()?)
            .subject(subject)
            .multipart(MultiPart::alternative_plain_html(plain, html))?;

        self.transport.send(email).await?;
        Ok(())
    }

    /// `is_new_user` picks the CTA copy: an existing account links straight to
    /// the page, a not-yet-registered email is told to sign up first (the
    /// share resolves automatically once they do — see `auth::register`'s
    /// pending-permission backfill).
    pub async fn send_share_notification(
        &self,
        to: &str,
        inviter_email: &str,
        page_title: &str,
        page_url: &str,
        is_new_user: bool,
    ) -> anyhow::Result<()> {
        let name = &self.product_name;
        let subject = format!("{inviter_email} shared \"{page_title}\" with you on {name}");

        let (plain, html) = if is_new_user {
            (
                format!(
                    "{inviter_email} shared \"{page_title}\" with you on {name}.\n\nCreate an account with this email address ({to}) and it'll be waiting for you:\n{page_url}"
                ),
                share_html(inviter_email, page_title, page_url, "Sign up to view it — it'll be waiting for you", name),
            )
        } else {
            (
                format!("{inviter_email} shared \"{page_title}\" with you on {name}.\n\nOpen it: {page_url}"),
                share_html(inviter_email, page_title, page_url, "Open the page", name),
            )
        };

        let email = Message::builder()
            .from(self.from.parse()?)
            .to(to.parse()?)
            .subject(subject)
            .multipart(MultiPart::alternative_plain_html(plain, html))?;

        self.transport.send(email).await?;
        Ok(())
    }
}

/// Inline-styled so it renders consistently across email clients (most strip
/// `<style>` blocks); mirrors the app's dark, indigo-accented look.
fn share_html(inviter_email: &str, page_title: &str, page_url: &str, cta: &str, product_name: &str) -> String {
    let inviter_email = escape_html(inviter_email);
    let page_title = escape_html(page_title);
    let product_name = escape_html(product_name);
    let inviter_email = inviter_email.as_str();
    let page_title = page_title.as_str();
    format!(
        r#"<!doctype html>
<html>
  <body style="margin:0;padding:32px 16px;background:#0f172a;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;">
    <table role="presentation" width="100%" cellpadding="0" cellspacing="0">
      <tr>
        <td align="center">
          <table role="presentation" width="480" cellpadding="0" cellspacing="0" style="max-width:480px;width:100%;background:#1e293b;border:1px solid #334155;border-radius:12px;overflow:hidden;">
            <tr>
              <td style="padding:32px 40px 8px;">
                <span style="font-size:18px;font-weight:700;color:#f1f5f9;letter-spacing:-0.02em;">{product_name}</span>
              </td>
            </tr>
            <tr>
              <td style="padding:8px 40px 0;">
                <p style="margin:0;font-size:15px;line-height:1.6;color:#cbd5e1;">
                  <strong style="color:#f1f5f9;">{inviter_email}</strong> shared a document with you:
                </p>
                <p style="margin:8px 0 0;font-size:17px;font-weight:600;color:#f1f5f9;">{page_title}</p>
              </td>
            </tr>
            <tr>
              <td style="padding:24px 40px 32px;">
                <a href="{page_url}" style="display:inline-block;background:#4f46e5;border-radius:8px;padding:12px 24px;color:#ffffff;font-size:15px;font-weight:600;text-decoration:none;">{cta}</a>
              </td>
            </tr>
          </table>
          <p style="margin:20px 0 0;font-size:12px;color:#64748b;">&copy; {year} {product_name}</p>
        </td>
      </tr>
    </table>
  </body>
</html>"#,
        inviter_email = inviter_email,
        page_title = page_title,
        page_url = page_url,
        cta = cta,
        product_name = product_name,
        year = chrono::Utc::now().format("%Y"),
    )
}

fn otp_html(action: &str, code: &str, product_name: &str) -> String {
    let product_name = escape_html(product_name);
    format!(
        r#"<!doctype html>
<html>
  <body style="margin:0;padding:32px 16px;background:#0f172a;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;">
    <table role="presentation" width="100%" cellpadding="0" cellspacing="0">
      <tr>
        <td align="center">
          <table role="presentation" width="480" cellpadding="0" cellspacing="0" style="max-width:480px;width:100%;background:#1e293b;border:1px solid #334155;border-radius:12px;overflow:hidden;">
            <tr>
              <td style="padding:32px 40px 8px;">
                <span style="font-size:18px;font-weight:700;color:#f1f5f9;letter-spacing:-0.02em;">{product_name}</span>
              </td>
            </tr>
            <tr>
              <td style="padding:8px 40px 0;">
                <p style="margin:0;font-size:15px;line-height:1.6;color:#cbd5e1;">
                  Use the code below to {action}.
                </p>
              </td>
            </tr>
            <tr>
              <td style="padding:28px 40px;">
                <div style="background:#4f46e5;border-radius:8px;padding:20px 24px;text-align:center;">
                  <span style="font-size:32px;font-weight:700;letter-spacing:0.3em;color:#ffffff;">{code}</span>
                </div>
              </td>
            </tr>
            <tr>
              <td style="padding:0 40px 32px;">
                <p style="margin:0;font-size:13px;line-height:1.6;color:#94a3b8;">
                  This code expires shortly and can only be used once. If you didn't request this, you can safely ignore this email.
                </p>
              </td>
            </tr>
          </table>
          <p style="margin:20px 0 0;font-size:12px;color:#64748b;">&copy; {year} {product_name}</p>
        </td>
      </tr>
    </table>
  </body>
</html>"#,
        action = action,
        code = code,
        product_name = product_name,
        year = chrono::Utc::now().format("%Y"),
    )
}

/// Welcome email body — used standalone (Google sign-up/login) and appended to
/// the registration OTP email. Mirrors the app's dark, indigo-accented look.
fn welcome_html(product_name: &str, cta: &str, app_url: &str) -> String {
    let product_name = escape_html(product_name);
    format!(
        r#"<!doctype html>
<html>
  <body style="margin:0;padding:32px 16px;background:#0f172a;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;">
    <table role="presentation" width="100%" cellpadding="0" cellspacing="0">
      <tr>
        <td align="center">
          <table role="presentation" width="480" cellpadding="0" cellspacing="0" style="max-width:480px;width:100%;background:#1e293b;border:1px solid #334155;border-radius:12px;overflow:hidden;">
            <tr>
              <td style="padding:32px 40px 8px;">
                <span style="font-size:18px;font-weight:700;color:#f1f5f9;letter-spacing:-0.02em;">{product_name}</span>
              </td>
            </tr>
            <tr>
              <td style="padding:8px 40px 0;">
                <p style="margin:0;font-size:15px;line-height:1.6;color:#cbd5e1;">
                  Welcome to {product_name}! Your account is ready.
                </p>
              </td>
            </tr>
            <tr>
              <td style="padding:24px 40px 32px;">
                <a href="{app_url}" style="display:inline-block;background:#4f46e5;border-radius:8px;padding:12px 24px;color:#ffffff;font-size:15px;font-weight:600;text-decoration:none;">{cta}</a>
              </td>
            </tr>
          </table>
          <p style="margin:20px 0 0;font-size:12px;color:#64748b;">&copy; {year} {product_name}</p>
        </td>
      </tr>
    </table>
  </body>
</html>"#,
        product_name = product_name,
        cta = cta,
        app_url = app_url,
        year = chrono::Utc::now().format("%Y"),
    )
}

/// Registration OTP email with the welcome message above the code block.
fn register_otp_welcome_html(code: &str, product_name: &str) -> String {
    let product_name = escape_html(product_name);
    format!(
        r#"<!doctype html>
<html>
  <body style="margin:0;padding:32px 16px;background:#0f172a;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;">
    <table role="presentation" width="100%" cellpadding="0" cellspacing="0">
      <tr>
        <td align="center">
          <table role="presentation" width="480" cellpadding="0" cellspacing="0" style="max-width:480px;width:100%;background:#1e293b;border:1px solid #334155;border-radius:12px;overflow:hidden;">
            <tr>
              <td style="padding:32px 40px 8px;">
                <span style="font-size:18px;font-weight:700;color:#f1f5f9;letter-spacing:-0.02em;">{product_name}</span>
              </td>
            </tr>
            <tr>
              <td style="padding:8px 40px 0;">
                <p style="margin:0;font-size:15px;line-height:1.6;color:#cbd5e1;">
                  Welcome to {product_name}! Verify your email with the code below.
                </p>
              </td>
            </tr>
            <tr>
              <td style="padding:28px 40px;">
                <div style="background:#4f46e5;border-radius:8px;padding:20px 24px;text-align:center;">
                  <span style="font-size:32px;font-weight:700;letter-spacing:0.3em;color:#ffffff;">{code}</span>
                </div>
              </td>
            </tr>
            <tr>
              <td style="padding:0 40px 32px;">
                <p style="margin:0;font-size:13px;line-height:1.6;color:#94a3b8;">
                  This code expires shortly and can only be used once. If you didn't request this, you can safely ignore this email.
                </p>
              </td>
            </tr>
          </table>
          <p style="margin:20px 0 0;font-size:12px;color:#64748b;">&copy; {year} {product_name}</p>
        </td>
      </tr>
    </table>
  </body>
</html>"#,
        code = code,
        product_name = product_name,
        year = chrono::Utc::now().format("%Y"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_html_escapes_user_controlled_values() {
        let html = share_html(
            "attacker@example.com",
            "<img src=x onerror=alert(document.cookie)>",
            "https://example.com/app/page",
            "Open the page",
            "me-doc",
        );
        assert!(!html.contains("<img src=x"));
        assert!(html.contains("&lt;img src=x onerror=alert(document.cookie)&gt;"));
    }

    #[test]
    fn register_otp_welcome_html_includes_the_code_and_escapes_product_name() {
        let html = register_otp_welcome_html("123456", "My <App>");
        assert!(html.contains(">123456<"));
        assert!(html.contains("My &lt;App&gt;"));
        assert!(!html.contains("My <App>"));
    }

    #[test]
    fn welcome_html_escapes_product_name_and_links_app_url() {
        let html = welcome_html("My <App>", "Open the app", "https://app.example.com");
        assert!(html.contains("My &lt;App&gt;"));
        assert!(html.contains("href=\"https://app.example.com\""));
        assert!(!html.contains("My <App>"));
    }
}
