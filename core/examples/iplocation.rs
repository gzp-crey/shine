use clap::{App, Arg, SubCommand};
use shine_core::iplocation::{
    IpCachedLocation, IpCachedLocationConfig, IpLocation, IpLocationError, IpLocationIpDataCo,
    IpLocationIpDataCoConfig, IpLocationProvider,
};
use std::net::IpAddr;
use std::time::Duration;

async fn query(provider: Box<dyn IpLocationProvider>, ips: Vec<IpAddr>) -> Vec<Result<IpLocation, IpLocationError>> {
    let mut locs = Vec::new();
    for ip in ips {
        locs.push(provider.get_location(&ip).await);
    }

    locs
}

fn main() {
    pretty_env_logger::init();

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
        .subcommand(
            SubCommand::with_name("cached_ipdataco")
                .about("Use ipdata.co")
                .arg(
                    Arg::with_name("key")
                        .short("k")
                        .long("key")
                        .takes_value(true)
                        .help("Sets the api key"),
                )
                .arg(
                    Arg::with_name("storage_account")
                        .short("a")
                        .long("storage_account")
                        .takes_value(true)
                        .help("Sets the storage account"),
                )
                .arg(
                    Arg::with_name("storage_account_secret")
                        .short("s")
                        .long("storage_account_secret")
                        .takes_value(true)
                        .help("Sets the storage account key"),
                ),
        )
        .get_matches();

    let mut rt = tokio::runtime::Runtime::new().unwrap();

    let provider: Box<dyn IpLocationProvider> = if let Some(matches) = matches.subcommand_matches("ipdataco") {
        let key = matches.value_of("key").unwrap();
        let cfg = IpLocationIpDataCoConfig {
            api_key: key.to_owned(),
        };
        Box::new(IpLocationIpDataCo::new(cfg))
    } else if let Some(matches) = matches.subcommand_matches("cached_ipdataco") {
        let key = matches.value_of("key").unwrap();
        let storage_account = matches.value_of("storage_account").unwrap().to_owned();
        let storage_account_secret = matches.value_of("storage_account_secret").unwrap().to_owned();
        let cfg = IpLocationIpDataCoConfig {
            api_key: key.to_owned(),
        };
        let provider = IpLocationIpDataCo::new(cfg);
        let cfg = IpCachedLocationConfig {
            storage_account: storage_account,
            storage_account_key: storage_account_secret,
            table_name: "ipcache".to_owned(),
            time_to_live: Duration::from_secs(12 * 60 * 60),
        };
        Box::new(rt.block_on(IpCachedLocation::new(provider, cfg)).unwrap())
    } else {
        return eprintln!("invalid subcommand");
    };

    let ips: Vec<IpAddr> = vec!["127.0.0.1", "104.215.148.63", "8.8.8.8", "2a00:1450:400d:805::2005"]
        .iter()
        .map(|x| x.parse().unwrap())
        .collect();
    println!("runing queries for: {:?}", ips);

    let loc = rt.block_on(query(provider, ips));

    println!("locations: {:#?}", loc);
}
