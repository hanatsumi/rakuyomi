use chrono::NaiveDateTime;

pub fn timestamp_f64(date: NaiveDateTime) -> f64 {
    date.timestamp() as f64 + (date.timestamp_subsec_nanos() as f64) / (10f64.powi(9))   
}
