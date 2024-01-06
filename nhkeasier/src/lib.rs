mod database;
mod email;
mod epub;
mod logging;
mod router;
mod ruby;
mod update;

pub use database::{connect_to_database, Story};
pub use email::{send_email_async, send_email_sync};
pub use epub::make_epub;
pub use logging::init_logging;
pub use router::{router, State};
pub use ruby::remove_ruby;
pub use update::update_stories;

use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone};

lazy_static::lazy_static! {
    pub static ref JST: FixedOffset = FixedOffset::east_opt(3600 * 9).unwrap();
}

pub fn parse_datetime_nhk(s: &str) -> DateTime<FixedOffset> {
    let dt = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap();
    JST.from_local_datetime(&dt).unwrap()
}

#[test]
fn test_parse_datetime_nhk() {
    assert_eq!(
        parse_datetime_nhk("2023-12-25 15:45:00"),
        DateTime::parse_from_rfc3339("2023-12-25T15:45:00+09:00").unwrap(),
    );
    assert_eq!(
        parse_datetime_nhk("2023-12-25 15:45:00"),
        DateTime::parse_from_rfc3339("2023-12-25T06:45:00+00:00").unwrap(),
    );
}
