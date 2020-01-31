use clap::{App, Arg, SubCommand};
use shine_core::iplocation::{IpLocation, IpLocationError, IpLocationIpDataCo, IpLocationIpDataCoConfig, IpLocationProvider};
use std::env;
use std::net::IpAddr;

async fn query<P: IpLocationProvider>(provider: P, ips: Vec<IpAddr>) -> Vec<Result<IpLocation, IpLocationError>> {
    let mut locs = Vec::new();
    for ip in ips {
        locs.push(provider.get_location(ip).await);
    }

    locs
}

fn main() {
    let matches = App::new("test ip location")
        .version("1.0")
        .subcommand(
            SubCommand::with_name("ipdataco").about("Use ipdata.co").arg(
                Arg::with_name("key")
                    .short("k")
                    .long("key")
                    .takes_value(true)
                    .help("Sets the api key"),
            ),
        )
        .get_matches();

    let provider = if let Some(matches) = matches.subcommand_matches("ipdataco") {
        let key = matches.value_of("key").unwrap();
        let cfg = IpLocationIpDataCoConfig { api_key: key.to_owned() };
        IpLocationIpDataCo::new(cfg)
    } else {
        return eprintln!("invalid subcommand");
    };

    let ips: Vec<IpAddr> = vec!["127.0.0.1", "62.77.220.46"].iter().map(|x| x.parse().unwrap()).collect();
    println!("runing queries for: {:?}", ips);

    let mut rt = tokio::runtime::Runtime::new().unwrap();
    let loc = rt.block_on(query(provider, ips));

    println!("locations: {:#?}", loc);
}
