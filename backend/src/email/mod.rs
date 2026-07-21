use lettre::message::MultiPart;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

#[derive(Clone)]
pub struct EmailClient {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
}

impl EmailClient {
    pub fn new(host: &str, port: u16, from: &str) -> anyhow::Result<Self> {
        let transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
            .port(port)
            .build();
        Ok(Self {
            transport,
            from: from.to_string(),
        })
    }

    pub async fn send_otp(&self, to: &str, purpose: &str, code: &str) -> anyhow::Result<()> {
        let action = match purpose {
            "register" => "verify your email",
            "login" => "complete your login",
            _ => "continue",
        };
        let subject = "Your me-doc verification code";

        let plain = format!(
            "Use this code to {action}: {code}\n\nThis code expires shortly and can only be used once.\nIf you didn't request this, you can ignore this email."
        );
        let html = otp_html(action, code);

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
        let subject = format!("{inviter_email} shared \"{page_title}\" with you on me-doc");

        let (plain, html) = if is_new_user {
            (
                format!(
                    "{inviter_email} shared \"{page_title}\" with you on me-doc.\n\nCreate an account with this email address ({to}) and it'll be waiting for you:\n{page_url}"
                ),
                share_html(inviter_email, page_title, page_url, "Sign up to view it — it'll be waiting for you"),
            )
        } else {
            (
                format!("{inviter_email} shared \"{page_title}\" with you on me-doc.\n\nOpen it: {page_url}"),
                share_html(inviter_email, page_title, page_url, "Open the page"),
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
fn share_html(inviter_email: &str, page_title: &str, page_url: &str, cta: &str) -> String {
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
                <span style="font-size:18px;font-weight:700;color:#f1f5f9;letter-spacing:-0.02em;">me-doc</span>
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
          <p style="margin:20px 0 0;font-size:12px;color:#64748b;">&copy; {year} me-doc</p>
        </td>
      </tr>
    </table>
  </body>
</html>"#,
        inviter_email = inviter_email,
        page_title = page_title,
        page_url = page_url,
        cta = cta,
        year = chrono::Utc::now().format("%Y"),
    )
}

fn otp_html(action: &str, code: &str) -> String {
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
                <span style="font-size:18px;font-weight:700;color:#f1f5f9;letter-spacing:-0.02em;">me-doc</span>
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
          <p style="margin:20px 0 0;font-size:12px;color:#64748b;">&copy; {year} me-doc</p>
        </td>
      </tr>
    </table>
  </body>
</html>"#,
        action = action,
        code = code,
        year = chrono::Utc::now().format("%Y"),
    )
}
