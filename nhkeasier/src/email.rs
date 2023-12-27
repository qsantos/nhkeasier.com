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
            "NHKEasier <bugs@nhkeasier.com>"
                .parse()
                .expect("failed to parse from address"),
        )
        .to("NHKEasier <contact@nhkeasier.com>"
            .parse()
            .expect("failed to parse to address"))
        .subject(format!("[NHKEasier] {}", subject))
        .header(ContentType::TEXT_PLAIN)
        .body(body)
        .expect("failed to create email message");

    let creds = Credentials::new(user, password);
    (message, host, creds)
}

pub fn send_email_sync(subject: &str, body: String) {
    let (message, host, creds) = send_email_common(subject, body);
    let mailer = SmtpTransport::relay(&host)
        .unwrap()
        .credentials(creds)
        .build();
    mailer.send(&message).unwrap();
}

pub async fn send_email_async(subject: &str, body: String) {
    let (message, host, creds) = send_email_common(subject, body);
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
        .unwrap()
        .credentials(creds)
        .build();
    mailer.send(message).await.unwrap();
}
