use crate::opts::ServeArgs;
use crate::serve::DumpError;
use log::info;
use poem::Result;
use poem::{error::Forbidden, Request};
use std::collections::HashSet;
use std::net::IpAddr;
use std::str::FromStr;

use poem::{async_trait, Endpoint, Middleware};
pub struct DenyIps {
    ips: HashSet<IpAddr>,
}

impl<E: Endpoint> Middleware<E> for DenyIps {
    type Output = DenyIpsImpl<E>;

    fn transform(&self, ep: E) -> Self::Output {
        DenyIpsImpl(self.ips.clone(), ep)
    }
}

pub struct DenyIpsImpl<E>(HashSet<IpAddr>, E);

#[async_trait]
impl<E: Endpoint> Endpoint for DenyIpsImpl<E> {
    type Output = <E as Endpoint>::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let remote_ip = req.remote_addr().0.as_socket_addr().unwrap().ip();
        if self.0.contains(&remote_ip) {
            info!("Blocked IP: {}", remote_ip);
            return Err(Forbidden(DumpError::new("Forbidden".to_string())));
        }
        self.1.call(req).await
    }
}

pub fn build_deny_ips(args: &ServeArgs) -> DenyIps {
    let mut ips = HashSet::new();
    if args.blocked_ips.is_none() {
        return DenyIps { ips };
    }
    let blocked_ips_file =
        std::fs::read_to_string(&args.blocked_ips.clone().unwrap()).expect("Could not read file");
    for mut ip_str in blocked_ips_file.lines() {
        ip_str = ip_str.trim();
        let ip = IpAddr::from_str(&ip_str).expect("Could not parse IP");
        ips.insert(ip);
    }
    DenyIps { ips }
}
