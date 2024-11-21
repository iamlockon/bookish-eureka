use chrono::DateTime;

pub(crate) mod helper {
    #[cfg(not(test))]
    pub use super::get_utc_now;
    #[cfg(test)]
    pub use super::mock_chrono::get_utc_now;
}

#[cfg(test)]
mod mock_chrono {
    use chrono::DateTime;
    use std::cell::Cell;

    thread_local! {
        static MOCK_NOW: Cell<i64> = const { Cell::new(0) };
    }

    #[cfg(test)]
    pub fn get_utc_now() -> DateTime<chrono::Utc> {
        MOCK_NOW
            .with(|now| DateTime::<chrono::Utc>::from_timestamp(now.get(), 0))
            .expect("invalid timestamp")
    }
}

#[cfg(not(test))]
pub fn get_utc_now() -> DateTime<chrono::Utc> {
    chrono::Utc::now()
}
