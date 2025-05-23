mod database;
mod email;
mod epub;
mod logging;
mod router;
mod ruby;
mod update;

use std::sync::LazyLock;

pub use database::{Story, connect_to_database};
pub use email::{send_email_async, send_email_sync};
pub use epub::make_epub;
pub use logging::init_logging;
pub use router::{State, router};
pub use ruby::remove_ruby;
pub use update::{extract_story_content, update_stories};

use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone};

#[cfg(debug_assertions)]
pub const DEBUG: bool = true;
#[cfg(not(debug_assertions))]
pub const DEBUG: bool = false;

pub static JST: LazyLock<FixedOffset> = LazyLock::new(|| FixedOffset::east_opt(3600 * 9).unwrap());

pub fn parse_datetime_nhk(s: &str) -> DateTime<FixedOffset> {
    let dt =
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").expect("failed to parse datetime");
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
