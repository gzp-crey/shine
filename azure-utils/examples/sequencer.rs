use azure_utils::idgenerator::{IdSequence, IdSequenceError, SyncCounterConfig, SyncCounterStore};
use clap::{App, Arg, SubCommand};
use std::collections::HashSet;
use std::env;
use std::str::FromStr;

#[derive(Clone)]
enum Method {
    Store,
    Sequence(u64),
}

async fn store_counter(
    t: usize,
    cfg: SyncCounterConfig,
    per_thread_count: usize,
) -> Result<(usize, Vec<String>), IdSequenceError> {
    let counter = SyncCounterStore::new(cfg).await?;
    let mut ids = vec![];

    for c in 0..per_thread_count {
        match counter.get("cnt").await {
            Ok(id) => ids.push(id.to_string()),
            Err(e) => println!("{},{} -> {:?}", t, c, e),
        };
    }

    Ok((t, ids))
}

async fn sequence_counter(
    t: usize,
    cfg: SyncCounterConfig,
    per_thread_count: usize,
    granularity: u64,
) -> Result<(usize, Vec<String>), IdSequenceError> {
    let counter = SyncCounterStore::new(cfg).await?;
    let counter = IdSequence::new(counter, "cnt").with_granularity(granularity);
    let mut ids = vec![];

    for c in 0..per_thread_count {
        match counter.get().await {
            Ok(id) => ids.push(id.to_string()),
            Err(e) => println!("{},{} -> {:?}", t, c, e),
        };
    }

    Ok((t, ids))
}

fn main() {
    let cfg = SyncCounterConfig {
        storage_account: env::var("STORAGE_ACCOUNT").expect("Missing STORAGE_ACCOUNT env variable"),
        storage_account_key: env::var("STORAGE_ACCOUNT_KEY").expect("Missing STORAGE_ACCOUNT_KEY env variable"),
        table_name: "testcounter".to_string(),
    };

    const THREAD_COUNT: usize = 4;
    const PER_THREAD_COUNT: usize = 100;

    let matches = App::new("test sequences")
        .version("1.0")
        .subcommand(SubCommand::with_name("store").about("Use raw sync store"))
        .subcommand(
            SubCommand::with_name("sequence").about("Use idsequencer").arg(
                Arg::with_name("granularity")
                    .short("g")
                    .long("granularity")
                    .help("Sets the allocation granularity"),
            ),
        )
        .get_matches();

    let method = if let Some(_) = matches.subcommand_matches("store") {
        Method::Store
    } else if let Some(matches) = matches.subcommand_matches("sequence") {
        Method::Sequence(u64::from_str(matches.value_of("granularity").unwrap_or("10")).unwrap())
    } else {
        return eprintln!("invalid subcommand");
    };

    let mut th = vec![];
    for t in 0..THREAD_COUNT {
        let cfg = cfg.clone();
        let method = method.clone();
        th.push(std::thread::spawn(move || {
            let mut rt = tokio::runtime::Runtime::new().unwrap();
            match method {
                Method::Store => rt.block_on(store_counter(t, cfg, PER_THREAD_COUNT)),
                Method::Sequence(g) => rt.block_on(sequence_counter(t, cfg, PER_THREAD_COUNT, g)),
            }
        }));
    }

    let mut ids = vec![];
    for e in th {
        match e.join().expect("Failed to join thread") {
            Ok((_, mut i)) => {
                ids.append(&mut i);
            }
            Err(e) => {
                println!("thread error: {:?}", e);
            }
        }
    }
    ids.sort();

    println!(
        "range: {}..{}, count: {}",
        ids.first().unwrap(),
        ids.last().unwrap(),
        ids.len()
    );

    let mut ui = HashSet::new();
    for c in &ids {
        if !ui.insert(c.clone()) {
            println!("duplicated id: {}", c);
        }
    }
}
