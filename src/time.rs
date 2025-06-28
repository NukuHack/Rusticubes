use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use serde::{Serializer, Deserializer};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time {
    pub year: u16,
    pub month: u8,    // 1-12
    pub day: u8,      // 1-31
    pub hour: u8,     // 0-23
    pub minute: u8,   // 0-59
    pub second: u8,   // 0-59
    pub day_of_year: u16, // 1-366 (day counter)
}

impl Serialize for Time {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Time {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Time::from_str(&s).map_err(serde::de::Error::custom)
    }
}
#[allow(dead_code)]
impl Time {
    pub fn now() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap();
        
        Self::from_unix_timestamp(now.as_secs())
    }

    pub fn from_unix_timestamp(timestamp: u64) -> Self {
        const SECONDS_PER_DAY: u64 = 86400;
        let mut remaining_seconds = timestamp;
        
        // Calculate year and day of year
        let mut year = 1970;
        while remaining_seconds >= SECONDS_PER_DAY * Self::days_in_year_(year) {
            remaining_seconds -= SECONDS_PER_DAY * Self::days_in_year_(year);
            year += 1;
        }
        
        let day_of_year = (remaining_seconds / SECONDS_PER_DAY) as u16 + 1;
        remaining_seconds %= SECONDS_PER_DAY;
        
        // Calculate month and day from day_of_year
        let (month, day) = Self::month_day_from_doy_(day_of_year, year);
        
        // Calculate time components
        let hour = (remaining_seconds / 3600) as u8;
        remaining_seconds %= 3600;
        let minute = (remaining_seconds / 60) as u8;
        let second = (remaining_seconds % 60) as u8;
        
        Time {
            year: year as u16,
            month,
            day,
            hour,
            minute,
            second,
            day_of_year,
        }
    }
    
    pub fn from_str(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split(&['.', ':', '-', '/', ' '][..]).collect();
        
        if parts.len() != 6 {
            return Err("Invalid time format - expected 6 components".to_string());
        }
        
        let year = parts[0].parse().map_err(|e| format!("Invalid year: {}", e))?;
        let month = parts[1].parse().map_err(|e| format!("Invalid month: {}", e))?;
        let day = parts[2].parse().map_err(|e| format!("Invalid day: {}", e))?;
        let hour = parts[3].parse().map_err(|e| format!("Invalid hour: {}", e))?;
        let minute = parts[4].parse().map_err(|e| format!("Invalid minute: {}", e))?;
        let second = parts[5].parse().map_err(|e| format!("Invalid second: {}", e))?;
        
        // Validate ranges
        if month == 0 || month > 12 {
            return Err("Month must be between 1-12".to_string());
        }
        if day == 0 || day > Self::days_in_month_(month, year as i32) {
            return Err(format!("Day must be between 1-{} for month {}", Self::days_in_month_(month, year as i32), month));
        }
        if hour > 23 {
            return Err("Hour must be between 0-23".to_string());
        }
        if minute > 59 {
            return Err("Minute must be between 0-59".to_string());
        }
        if second > 59 {
            return Err("Second must be between 0-59".to_string());
        }
        
        let day_of_year = Self::calculate_day_of_year_(day, month, year as i32);
        
        Ok(Time {
            year,
            month,
            day,
            hour,
            minute,
            second,
            day_of_year,
        })
    }
    
    pub fn to_unix_timestamp(&self) -> u64 {
        let mut total_days = 0;
        
        // Add days from all previous years
        for year in 1970..self.year {
            total_days += Self::days_in_year_(year as i32) as u64;
        }
        
        // Add days from current year (using day_of_year for more reliable calculation)
        total_days += (self.day_of_year - 1) as u64;
        
        // Calculate total seconds
        total_days * 86400 
            + self.hour as u64 * 3600 
            + self.minute as u64 * 60 
            + self.second as u64
    }
    
    pub fn day_of_week(&self) -> u8 {
        // Zeller's Congruence algorithm to calculate day of week (0=Sunday, 6=Saturday)
        let mut m = self.month as i32;
        let mut y = self.year as i32;
        if m < 3 {
            m += 12;
            y -= 1;
        }
        let k = y % 100;
        let j = y / 100;
        let h = (self.day as i32 + 13*(m+1)/5 + k + k/4 + j/4 + 5*j) % 7;
        ((h + 6) % 7) as u8 // Convert to 0=Monday, 6=Sunday
    }
    
    pub fn weekday_name(&self) -> &'static str {
        match self.day_of_week() {
            0 => "Monday",
            1 => "Tuesday",
            2 => "Wednesday",
            3 => "Thursday",
            4 => "Friday",
            5 => "Saturday",
            6 => "Sunday",
            _ => unreachable!(),
        }
    }
    
    pub fn is_leap_year(&self) -> bool {
        Self::is_leap_year_(self.year as i32)
    }
    
    pub fn days_in_month(&self) -> u8 {
        Self::days_in_month_(self.month, self.year as i32)
    }

    // Helper functions
    fn days_in_month_(month: u8, year: i32) -> u8 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => if Self::is_leap_year_(year) { 29 } else { 28 },
            _ => 0, // invalid month
        }
    }

    fn days_in_year_(year: i32) -> u64 {
        if Self::is_leap_year_(year) { 366 } else { 365 }
    }

    fn is_leap_year_(year: i32) -> bool {
        year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
    }

    fn calculate_day_of_year_(day: u8, month: u8, year: i32) -> u16 {
        let mut doy = day as u16;
        for m in 1..month {
            doy += Self::days_in_month_(m, year) as u16;
        }
        doy
    }

    fn month_day_from_doy_(mut day_of_year: u16, year: i32) -> (u8, u8) {
        let mut month = 1;
        while month <= 12 {
            let days_in_month = Self::days_in_month_(month, year) as u16;
            if day_of_year <= days_in_month {
                break;
            }
            day_of_year -= days_in_month;
            month += 1;
        }
        (month, day_of_year as u8)
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f, 
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}", 
            self.year, 
            self.month, 
            self.day, 
            self.hour, 
            self.minute, 
            self.second
        )
    }
}
