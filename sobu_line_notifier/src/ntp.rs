use embassy_net::{Stack, dns::DnsQueryType};
use embassy_time::{Duration, Timer};

const NTP_SERVER: &str = "ntp.nict.jp";
const NTP_PORT: u16 = 123;
const NTP_PACKET_SIZE: usize = 48;
const UNIX_EPOCH_OFFSET: u64 = 2208988800; // Seconds between 1900 and 1970

pub struct NtpTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl NtpTime {
    pub fn to_seconds_since_epoch(&self) -> u64 {
        let mut days = 0u64;

        // Add days for complete years
        for year in 1970..self.year {
            days += if is_leap_year(year) { 366 } else { 365 };
        }

        // Add days for complete months in current year
        const DAYS_IN_MONTH: [u16; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        const DAYS_IN_MONTH_LEAP: [u16; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

        let month_days = if is_leap_year(self.year) {
            &DAYS_IN_MONTH_LEAP
        } else {
            &DAYS_IN_MONTH
        };

        for month in 0..(self.month as usize - 1) {
            days += month_days[month] as u64;
        }

        // Add remaining days
        days += (self.day - 1) as u64;

        // Convert to seconds and add time
        days * 86400 + (self.hour as u64 * 3600) + (self.minute as u64 * 60) + self.second as u64
    }

    pub fn from_seconds_since_epoch(timestamp: u64) -> Self {
        // timestamp is already in JST, no adjustment needed
        timestamp_to_datetime(timestamp)
    }

    pub fn add_seconds(&self, seconds: u64) -> Self {
        let epoch_seconds = self.to_seconds_since_epoch();
        let new_epoch = epoch_seconds + seconds;
        Self::from_seconds_since_epoch(new_epoch)
    }
}

pub async fn get_ntp_time(stack: &Stack<'static>) -> Result<NtpTime, &'static str> {
    // Wait for network to be ready
    let mut retry_count = 0;
    while !stack.is_link_up() {
        if retry_count > 30 {
            return Err("Network not ready");
        }
        Timer::after(Duration::from_secs(1)).await;
        retry_count += 1;
    }

    // Wait for IP address
    let mut retry_count = 0;
    while stack.config_v4().is_none() {
        if retry_count > 30 {
            return Err("No IP address");
        }
        Timer::after(Duration::from_secs(1)).await;
        retry_count += 1;
    }

    log::info!("Resolving NTP server: {}", NTP_SERVER);

    // DNS lookup
    let addrs = match stack.dns_query(NTP_SERVER, DnsQueryType::A).await {
        Ok(addrs) => addrs,
        Err(e) => {
            log::error!("DNS lookup failed: {:?}", e);
            return Err("DNS lookup failed");
        }
    };

    if addrs.is_empty() {
        return Err("DNS resolution returned no addresses");
    }

    let remote_endpoint = embassy_net::IpEndpoint::new(addrs[0], NTP_PORT);

    log::info!("NTP server resolved to: {:?}", remote_endpoint);

    // Create UDP socket
    let mut rx_meta = [embassy_net::udp::PacketMetadata::EMPTY; 1];
    let mut rx_buffer = [0; NTP_PACKET_SIZE];
    let mut tx_meta = [embassy_net::udp::PacketMetadata::EMPTY; 1];
    let mut tx_buffer = [0; NTP_PACKET_SIZE];

    let mut socket = embassy_net::udp::UdpSocket::new(
        *stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );

    socket.bind(0).map_err(|_| "Failed to bind UDP socket")?;

    // Prepare NTP request packet
    let mut ntp_packet = [0u8; NTP_PACKET_SIZE];
    ntp_packet[0] = 0x1b; // LI = 0, VN = 3, Mode = 3 (client)

    log::info!("Sending NTP request to {:?}", remote_endpoint);

    // Send NTP request
    socket
        .send_to(&ntp_packet, remote_endpoint)
        .await
        .map_err(|_| "Failed to send NTP request")?;

    // Receive NTP response with timeout
    let mut response = [0u8; NTP_PACKET_SIZE];
    let recv_result = embassy_time::with_timeout(Duration::from_secs(5), async {
        socket.recv_from(&mut response).await
    })
    .await;

    match recv_result {
        Ok(Ok((len, _from))) => {
            log::info!("Received NTP response: {} bytes", len);

            // Extract timestamp from response (bytes 40-43)
            let timestamp =
                u32::from_be_bytes([response[40], response[41], response[42], response[43]]);

            // Convert NTP timestamp to Unix timestamp
            let unix_timestamp = timestamp as u64 - UNIX_EPOCH_OFFSET;

            // Adjust for JST (UTC+9)
            let jst_timestamp = unix_timestamp + (9 * 3600);

            // Convert to date/time
            let ntp_time = timestamp_to_datetime(jst_timestamp);

            log::info!(
                "NTP Time (JST): {:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                ntp_time.year,
                ntp_time.month,
                ntp_time.day,
                ntp_time.hour,
                ntp_time.minute,
                ntp_time.second
            );

            Ok(ntp_time)
        }
        Ok(Err(_)) => {
            log::error!("Failed to receive NTP response");
            Err("Failed to receive NTP response")
        }
        Err(_) => {
            log::error!("NTP request timeout");
            Err("NTP request timeout")
        }
    }
}

fn timestamp_to_datetime(timestamp: u64) -> NtpTime {
    const SECONDS_PER_DAY: u64 = 86400;
    const DAYS_PER_YEAR: u64 = 365;
    const DAYS_PER_LEAP_YEAR: u64 = 366;

    let days_since_epoch = timestamp / SECONDS_PER_DAY;
    let seconds_today = timestamp % SECONDS_PER_DAY;

    let hour = (seconds_today / 3600) as u8;
    let minute = ((seconds_today % 3600) / 60) as u8;
    let second = (seconds_today % 60) as u8;

    // Simple year calculation (starting from 1970)
    let mut year = 1970u16;
    let mut remaining_days = days_since_epoch;

    loop {
        let days_in_year = if is_leap_year(year) {
            DAYS_PER_LEAP_YEAR
        } else {
            DAYS_PER_YEAR
        };

        if remaining_days < days_in_year {
            break;
        }

        remaining_days -= days_in_year;
        year += 1;
    }

    // Calculate month and day
    let (month, day) = days_to_month_day(remaining_days as u16, is_leap_year(year));

    NtpTime {
        year,
        month,
        day,
        hour,
        minute,
        second,
    }
}

fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn days_to_month_day(days: u16, is_leap: bool) -> (u8, u8) {
    const DAYS_IN_MONTH: [u16; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    const DAYS_IN_MONTH_LEAP: [u16; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    let month_days = if is_leap {
        &DAYS_IN_MONTH_LEAP
    } else {
        &DAYS_IN_MONTH
    };

    let mut remaining = days + 1; // Days are 1-indexed
    for (i, &days_in_month) in month_days.iter().enumerate() {
        if remaining <= days_in_month {
            return ((i + 1) as u8, remaining as u8);
        }
        remaining -= days_in_month;
    }

    (12, 31) // Fallback
}
