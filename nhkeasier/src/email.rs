use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, SmtpTransport, Tokio1Executor, Transport,
};

fn send_email_common(subject: &str, body: String) -> (Message, String, Credentials) {
    let host = std::env::var("EMAIL_HOST").expect("missing environment variable EMAIL_HOST");
    let user = std::env::var("EMAIL_USER").expect("missing environment variable EMAIL_USER");
    let password =
        std::env::var("EMAIL_PASSWORD").expect("missing environment variable EMAIL_PASSWORD");

    let message = Message::builder()
        .from(
            "NHK Easier <bugs@nhkeasier.com>"
                .parse()
                .expect("failed to parse from address"),
        )
        .to("NHK Easier <contact@nhkeasier.com>"
            .parse()
            .expect("failed to parse to address"))
        .subject(format!("[NHK Easier] {subject}"))
        .header(ContentType::TEXT_PLAIN)
        .body(body)
        .expect("failed to create email message");

    let creds = Credentials::new(user, password);
    (message, host, creds)
}

pub fn send_email_sync(subject: &str, body: String) {
    let (message, host, creds) = send_email_common(subject, body);
    let mailer = SmtpTransport::relay(&host)
        .expect("failed to create email transport (sync)")
        .credentials(creds)
        .build();
    mailer.send(&message).expect("failed to send email (sync)");
}

pub async fn send_email_async(subject: &str, body: String) {
    let (message, host, creds) = send_email_common(subject, body);
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
        .expect("failed to create email transport (async)")
        .credentials(creds)
        .build();
    mailer
        .send(message)
        .await
        .expect("failed to send email (async)");
}
