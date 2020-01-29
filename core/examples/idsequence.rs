use clap::{App, Arg, SubCommand};
use shine_core::idgenerator::{IdSequence, IdSequenceError, SaltedIdSequence, SyncCounterConfig, SyncCounterStore};
use std::collections::HashSet;
use std::env;
use std::str::FromStr;

#[derive(Debug, Clone)]
enum Method {
    Store,
    Sequence(u64),
    SaltedSequence(u64),
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

async fn salted_sequence_counter(
    t: usize,
    cfg: SyncCounterConfig,
    per_thread_count: usize,
    granularity: u64,
) -> Result<(usize, Vec<String>), IdSequenceError> {
    let counter = SyncCounterStore::new(cfg).await?;
    let counter = SaltedIdSequence::new(counter, "cnt").with_granularity(granularity);
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
        starting_value: 0,
    };

    let matches = App::new("test sequences")
        .version("1.0")
        .arg(
            Arg::with_name("threads")
                .short("t")
                .long("threads")
                .default_value("4")
                .takes_value(true)
                .help("Number of threads"),
        )
        .arg(
            Arg::with_name("count")
                .short("c")
                .long("count")
                .default_value("100")
                .takes_value(true)
                .help("Number of id per thread"),
        )
        .subcommand(SubCommand::with_name("store").about("Use SyncCounterStore"))
        .subcommand(
            SubCommand::with_name("sequence").about("Use IdSequence").arg(
                Arg::with_name("granularity")
                    .short("g")
                    .long("granularity")
                    .default_value("10")
                    .takes_value(true)
                    .help("Sets the allocation granularity"),
            ),
        )
        .subcommand(
            SubCommand::with_name("salted").about("Use SaltedIdSequence").arg(
                Arg::with_name("granularity")
                    .short("g")
                    .long("granularity")
                    .default_value("10")
                    .takes_value(true)
                    .help("Sets the allocation granularity"),
            ),
        )
        .get_matches();

    let thread_count = usize::from_str(matches.value_of("threads").unwrap()).unwrap();
    let per_thread_count = usize::from_str(matches.value_of("count").unwrap()).unwrap();

    let method = if let Some(_) = matches.subcommand_matches("store") {
        Method::Store
    } else if let Some(matches) = matches.subcommand_matches("sequence") {
        Method::Sequence(u64::from_str(matches.value_of("granularity").unwrap()).unwrap())
    } else if let Some(matches) = matches.subcommand_matches("salted") {
        Method::SaltedSequence(u64::from_str(matches.value_of("granularity").unwrap()).unwrap())
    } else {
        return eprintln!("invalid subcommand");
    };

    println!(
        "Runing {:?} on {} threads with {} count",
        method, thread_count, per_thread_count
    );

    let mut th = vec![];
    for t in 0..thread_count {
        let cfg = cfg.clone();
        let method = method.clone();
        th.push(std::thread::spawn(move || {
            let mut rt = tokio::runtime::Runtime::new().unwrap();
            match method {
                Method::Store => rt.block_on(store_counter(t, cfg, per_thread_count)),
                Method::Sequence(g) => rt.block_on(sequence_counter(t, cfg, per_thread_count, g)),
                Method::SaltedSequence(g) => rt.block_on(salted_sequence_counter(t, cfg, per_thread_count, g)),
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
        "range: {}..{}, count: {}, errors: {}",
        ids.first().unwrap(),
        ids.last().unwrap(),
        ids.len(),
        thread_count * per_thread_count - ids.len()
    );

    let mut ui = HashSet::new();
    for c in &ids {
        if !ui.insert(c.clone()) {
            println!("duplicated id: {}", c);
        }
    }
}
