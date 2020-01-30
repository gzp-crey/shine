use clap::{App, Arg, SubCommand};
use shine_core::iplocation::{IpLocation, IpDataLocation, IpLocationError};
use std::collections::HashSet;
use std::env;
use std::str::FromStr;
use 

async fn query<P: IpLocationProvider>(provider: P, ip: Vec<IpAddr>) -> Vec<Result<IpLocation, IdLocationError>> {    
    let mut locs = Vec::new();
    for ip in ips {
        locs.push(provider.get_location(ip).await);
    }

    locs
}

fn main() {
    let cfg = SyncCounterConfig {
        storage_account: env::var("STORAGE_ACCOUNT").expect("Missing STORAGE_ACCOUNT env variable"),
        storage_account_key: env::var("STORAGE_ACCOUNT_KEY").expect("Missing STORAGE_ACCOUNT_KEY env variable"),
        table_name: "testcounter".to_string(),
        starting_value: 0,
    };

    let matches = App::new("test ip location")
        .version("1.0")
        .subcommand(
            SubCommand::with_name("ipdata").about("Use ipdata.co").arg(
                Arg::with_name("api_key")
                    .short("k")
                    .long("api_key")
                    .takes_value(true)
                    .help("Sets the api key"),
            ),
        )
        .get_matches();

    let provider = if let Some(_) = matches.subcommand_matches("ipdate") {
        let key = matches.value_of("api_key").unwrap();
        IpDataLocation::new(key)
    } else {
        return eprintln!("invalid subcommand");
    };

    let ips = vec![IpAddr(127.0.0.1)];

    rt.block_on(query_ipdata(provider, ips)),

    println!("locations: {:?}",loc);
}
