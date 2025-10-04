use crate::errors::{JitoClientError, JitoClientResult};
use std::fmt::{Display, Formatter};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy)]
pub enum NodeRegion {
    AM,
    DB,
    FRA,
    LN,
    NY,
    SLC,
    SG,
    TOK,
}

impl NodeRegion {
    const ALL: [NodeRegion; 8] = [
        NodeRegion::AM,
        NodeRegion::DB,
        NodeRegion::FRA,
        NodeRegion::LN,
        NodeRegion::NY,
        NodeRegion::SLC,
        NodeRegion::SG,
        NodeRegion::TOK,
    ];

    /// Pings each endpoint by performing a DNS resolution and establishing a TCP connection, and returns the endpoint with the fastest response time, along with the time (ms) it took.
    pub async fn measure_latency() -> JitoClientResult<(Self, Duration)> {
        /*let am_task = async { NodeRegion::AM.ping() };
        let db_task = async { NodeRegion::DB.ping() };
        let fra_task = async { NodeRegion::FRA.ping() };
        let ln_task = async { NodeRegion::LN.ping() };
        let ny_task = async { NodeRegion::NY.ping() };
        let slc_task = async { NodeRegion::SLC.ping() };
        let sg_task = async { NodeRegion::SG.ping() };
        let tok_task = async { NodeRegion::TOK.ping() };

        let (am_result, db_result, fra_result, ln_result, ny_result, slc_result, tok_result) =
            futures::join!(am_task, db_task, fra_task, ln_task, ny_task, slc_task, tok_task);

            let mut results = Vec::with_capacity(7);*/
        let tasks: Vec<_> = Self::ALL
            .iter()
            .map(|region| async move { (*region, region.ping()) })
            .collect();

        let results = futures::future::join_all(tasks).await;

        let mut successful_pings = Vec::new();
        for (region, result) in results {
            if let Ok(duration) = result {
                successful_pings.push((region, duration));
            }
        }

        let mut fastest = None;
        for (region, duration) in successful_pings {
            match fastest {
                None => fastest = Some((region, duration)),
                Some((_, best_duration)) if duration < best_duration => {
                    fastest = Some((region, duration));
                }
                _ => {}
            }
        }
        fastest.ok_or(JitoClientError::AllRegionLatencyMissing)

        /*
        match ny_result {
            Ok(dur) => results.push((NodeRegion::NY, dur)),
            Err(e) => return Err(e),
        }
        match dal_result {
            Ok(dur) => results.push((NodeRegion::DAL, dur)),
            Err(e) => return Err(e),
        }
        match slc_result {
            Ok(dur) => results.push((NodeRegion::SLC, dur)),
            Err(e) => return Err(e),
        }
        match results.into_iter().min_by_key(|(_, duration)| *duration) {
            Some(result) => Ok(result),
            None => Err(JitoClientError::AllRegionLatencyMissing),
        }
        */
    }

    // Attempts to perform a DNS resolution and establish a TCP connection, and returns the total execution time (ms)
    fn ping(&self) -> JitoClientResult<Duration> {
        let start = Instant::now();
        let addr = self
            .host()
            .to_socket_addrs()
            .map_err(|e| JitoClientError::DNSResolution(e))?
            .next()
            .ok_or(JitoClientError::DNSEmpty)?;
        let _ = TcpStream::connect_timeout(&addr, TIMEOUT)
            .map_err(|e| JitoClientError::TCPConnect(e))?;
        Ok(start.elapsed())
    }

    pub fn all() -> &'static [NodeRegion] {
        &Self::ALL
    }

    pub fn endpoint(&self) -> &'static str {
        match self {
            NodeRegion::AM => "https://amsterdam.mainnet.block-engine.jito.wtf:443",
            NodeRegion::DB => "https://dublin.mainnet.block-engine.jito.wtf:443",
            NodeRegion::FRA => "https://frankfurt.mainnet.block-engine.jito.wtf:443",
            NodeRegion::LN => "https://london.mainnet.block-engine.jito.wtf:443",
            NodeRegion::NY => "https://ny.mainnet.block-engine.jito.wtf:443",
            NodeRegion::SLC => "https://slc.mainnet.block-engine.jito.wtf:443",
            NodeRegion::SG => "https://singapore.mainnet.block-engine.jito.wtf:443",
            NodeRegion::TOK => "https://tokyo.mainnet.block-engine.jito.wtf:443",
        }
    }

    fn host(&self) -> &'static str {
        &self.endpoint()[8..]
    }
}

impl Display for NodeRegion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeRegion::AM => write!(f, "Amsterdam"),
            NodeRegion::DB => write!(f, "Dublin"),
            NodeRegion::FRA => write!(f, "Frankfurt"),
            NodeRegion::LN => write!(f, "London"),
            NodeRegion::NY => write!(f, "New York"),
            NodeRegion::SLC => write!(f, "Salt Lake City"),
            NodeRegion::SG => write!(f, "Singapore"),
            NodeRegion::TOK => write!(f, "Tokyo"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn print_all() {
        println!("All Node Regions:");
        for region in NodeRegion::ALL {
            println!(
                "Region: {}, URL: {}; ping: {} ms",
                region,
                region.endpoint(),
                region.ping().unwrap_or(Duration::from_secs(0)).as_millis()
            );
        }
    }

    #[tokio::test]
    #[serial]
    async fn measure_latency() {
        match NodeRegion::measure_latency().await {
            Ok(a) => println!("Lowest latency node: {}, {} ms", a.0, a.1.as_millis()),
            Err(e) => panic!("Measure latency failed: {e}"),
        }
    }
}
