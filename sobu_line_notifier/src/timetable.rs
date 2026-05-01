#![allow(dead_code)]

use heapless::Vec;

/// 時刻を表す構造体 (時:分)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
}

impl Time {
    pub const fn new(hour: u8, minute: u8) -> Self {
        Self { hour, minute }
    }

    pub const fn to_minutes(&self) -> u16 {
        self.hour as u16 * 60 + self.minute as u16
    }

    pub fn format(&self) -> heapless::String<5> {
        let mut s = heapless::String::new();
        let _ = core::fmt::write(&mut s, format_args!("{:02}:{:02}", self.hour, self.minute));
        s
    }
}

/// 総武線快速の時刻表データ (新小岩駅 → 東京/品川方面)
/// train.txtから取得した実際のダイヤ（平日）
pub const SOBU_RAPID_TIMETABLE: &[Time] = &[
    // 5時台
    Time::new(5, 10),
    Time::new(5, 31),
    // 6時台
    Time::new(6, 1),
    Time::new(6, 18),
    Time::new(6, 40),
    Time::new(6, 47),
    Time::new(6, 54),
    Time::new(6, 58),
    // 7時台
    Time::new(7, 10),
    Time::new(7, 15),
    Time::new(7, 20),
    Time::new(7, 25),
    Time::new(7, 31),
    Time::new(7, 35),
    Time::new(7, 38),
    Time::new(7, 42),
    Time::new(7, 48),
    Time::new(7, 51),
    Time::new(7, 54),
    Time::new(7, 57),
    // 8時台
    Time::new(8, 0),
    Time::new(8, 4),
    Time::new(8, 8),
    Time::new(8, 11),
    Time::new(8, 15),
    Time::new(8, 18),
    Time::new(8, 22),
    Time::new(8, 25),
    Time::new(8, 28),
    Time::new(8, 31),
    Time::new(8, 35),
    Time::new(8, 41),
    Time::new(8, 44),
    Time::new(8, 48),
    Time::new(8, 54),
    // 9時台
    Time::new(9, 0),
    Time::new(9, 3),
    Time::new(9, 12),
    Time::new(9, 17),
    Time::new(9, 26),
    Time::new(9, 35),
    Time::new(9, 49),
    Time::new(9, 57),
    // 10時台
    Time::new(10, 1),
    Time::new(10, 11),
    Time::new(10, 20),
    Time::new(10, 28),
    Time::new(10, 40),
    Time::new(10, 51),
    // 11時台
    Time::new(11, 9),
    Time::new(11, 18),
    Time::new(11, 28),
    Time::new(11, 39),
    Time::new(11, 51),
    // 12時台
    Time::new(12, 9),
    Time::new(12, 18),
    Time::new(12, 28),
    Time::new(12, 39),
    Time::new(12, 51),
    // 13時台
    Time::new(13, 9),
    Time::new(13, 18),
    Time::new(13, 28),
    Time::new(13, 40),
    Time::new(13, 51),
    // 14時台
    Time::new(14, 9),
    Time::new(14, 18),
    Time::new(14, 28),
    Time::new(14, 40),
    Time::new(14, 51),
    // 15時台
    Time::new(15, 9),
    Time::new(15, 18),
    Time::new(15, 28),
    Time::new(15, 40),
    Time::new(15, 52),
    // 16時台
    Time::new(16, 9),
    Time::new(16, 17),
    Time::new(16, 22),
    Time::new(16, 27),
    Time::new(16, 40),
    Time::new(16, 44),
    Time::new(16, 49),
    Time::new(16, 53),
    Time::new(16, 58),
    // 17時台
    Time::new(17, 3),
    Time::new(17, 12),
    Time::new(17, 20),
    Time::new(17, 27),
    Time::new(17, 39),
    Time::new(17, 44),
    Time::new(17, 48),
    Time::new(17, 56),
    // 18時台
    Time::new(18, 0),
    Time::new(18, 12),
    Time::new(18, 17),
    Time::new(18, 24),
    Time::new(18, 27),
    Time::new(18, 42),
    Time::new(18, 47),
    Time::new(18, 51),
    Time::new(18, 56),
    // 19時台
    Time::new(19, 1),
    Time::new(19, 4),
    Time::new(19, 13),
    Time::new(19, 22),
    Time::new(19, 26),
    Time::new(19, 30),
    Time::new(19, 40),
    Time::new(19, 45),
    Time::new(19, 51),
    Time::new(19, 57),
    // 20時台
    Time::new(20, 7),
    Time::new(20, 12),
    Time::new(20, 16),
    Time::new(20, 24),
    Time::new(20, 33),
    Time::new(20, 42),
    Time::new(20, 50),
    Time::new(20, 58),
    // 21時台
    Time::new(21, 9),
    Time::new(21, 16),
    Time::new(21, 23),
    Time::new(21, 34),
    Time::new(21, 41),
    Time::new(21, 49),
    Time::new(21, 54),
    // 22時台
    Time::new(22, 4),
    Time::new(22, 14),
    Time::new(22, 22),
    Time::new(22, 33),
    Time::new(22, 47),
    Time::new(22, 58),
    // 23時台
    Time::new(23, 8),
    Time::new(23, 20),
    Time::new(23, 28),
    Time::new(23, 36),
    Time::new(23, 52),
    // 0時台（翌日）
    Time::new(0, 15),
];

/// 現在時刻から次の2-3本の電車を取得
pub fn get_next_trains(current_time: Time, count: usize) -> Vec<Time, 3> {
    let mut next_trains = Vec::new();
    let current_minutes = current_time.to_minutes();

    for &train_time in SOBU_RAPID_TIMETABLE {
        if train_time.to_minutes() >= current_minutes {
            if next_trains.push(train_time).is_err() {
                break;
            }
            if next_trains.len() >= count {
                break;
            }
        }
    }

    next_trains
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_ordering() {
        let t1 = Time::new(7, 30);
        let t2 = Time::new(8, 15);
        assert!(t1 < t2);
    }

    #[test]
    fn test_get_next_trains() {
        let current = Time::new(7, 35);
        let trains = get_next_trains(current, 3);
        assert_eq!(trains.len(), 3);
        assert_eq!(trains[0], Time::new(7, 42));
        assert_eq!(trains[1], Time::new(7, 54));
        assert_eq!(trains[2], Time::new(8, 6));
    }
}
