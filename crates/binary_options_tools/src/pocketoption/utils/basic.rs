use chrono::{Duration, Utc};
use rand::{Rng, rng};

use crate::pocketoption::error::{PocketOptionError, PocketResult};

pub fn get_index() -> PocketResult<u64> {
    // rand = str(random.randint(10, 99))
    // cu = int(time.time())
    // t = str(cu + (2 * 60 * 60))
    // index = int(t + rand)
    let mut rng = rng();

    let rand = rng.random_range(10..99);
    let time = (Utc::now() + Duration::hours(2)).timestamp();
    format!("{time}{rand}")
        .parse::<u64>()
        .map_err(|e| PocketOptionError::GeneralParsingError(e.to_string()))
}

pub fn is_otc(symbol: &str) -> bool {
    symbol.ends_with("otc")
}
