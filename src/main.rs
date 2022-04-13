use clap::{crate_version, ArgEnum, Parser};

mod config;
mod report;
mod searchopts;

use crate::config::Config;
use crate::searchopts::SearchOptions;
use pestr::{Geometry, Reservation};

static CONFIG_FILE_NAME: &str = ".pestr.toml";

#[derive(Parser, Debug)]
#[clap(version = crate_version!(), author = "Andrew Dawson <andrew.dawson@ecmwf.int>")]
#[clap(about = "A PEs and threads calculator")]
struct Args {
    /// number of CPUs per node on the target machine
    #[clap(short = 'n', long, parse(try_from_str=positive_int))]
    cpus_per_node: Option<u32>,

    /// assume hyperthreading (doubles the effective CPUs per node)
    #[clap(short = 'y', long)]
    hyperthreading: bool,

    /// suggest alternative geometries that fill whole nodes
    #[clap(short, long)]
    search: Option<Option<String>>,

    // output format selection
    #[clap(arg_enum, short, long, default_value_t=Reporter::Text)]
    report_format: Reporter,

    /// configuration file path
    #[clap(short, long)]
    config_file: Option<String>,

    /// number of PEs (MPI tasks) allocated to the job
    #[clap(parse(try_from_str=positive_int))]
    pes: u32,

    /// number of threads allocated to the job
    #[clap(parse(try_from_str=positive_int))]
    threads: u32,
}

fn main() -> Result<(), String> {
    let args: Args = Args::parse();
    let config_file = match args.config_file {
        Some(config_file_path) => Some(shellexpand::tilde(&config_file_path).into_owned()),
        None => match dirs::home_dir() {
            Some(home) => home.join(CONFIG_FILE_NAME).to_str().map(|s| s.to_owned()),
            None => None,
        },
    };

    let config = match &config_file {
        Some(c) => Config::from_file(c),
        None => Config::new(),
    };

    let cpus_per_node = args.cpus_per_node.unwrap_or(config.cpus_per_node);

    // Construct the Geometry representing the user's job, and compute its reservation.
    let geom = Geometry::new(cpus_per_node, args.hyperthreading, args.pes, args.threads)
        .map_err(|e| format!("{}", e))?;

    let res = Reservation::from_geometry(geom);

    // Determine alternate geometries that yield a full reservation, within the
    // specified parameters. Use an empty list if the user didn't ask for
    // alternate geometries.
    let alternates = match args.search {
        None => Vec::new(),
        Some(search_option_str) => {
            let search_options = match search_option_str {
                None => SearchOptions::default(config.search), // FIXME: here we need to inject from our config
                Some(s) => SearchOptions::parse(&s, config.search)?, // FIXME: also here might need to know
            };

            let gr_filter = |_, r: Reservation| -> bool {
                if search_options.conserve_nodes {
                    r.nodes == res.nodes
                } else {
                    true
                }
            };
            geom.alternates(
                search_options.pe_radius,
                search_options.thread_radius,
                &gr_filter,
            )
        }
    };

    match args.report_format {
        Reporter::Text => report::text_reporter(res, alternates),
        Reporter::Json => report::json_reporter(geom, res, alternates),
    }
    Ok(())
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
enum Reporter {
    Json,
    Text,
}

fn positive_int(s: &str) -> Result<u32, String> {
    s.parse()
        .map_err(|_| String::from("must be a positive integer"))
        .and_then(|value| {
            if value == 0 {
                Err(String::from("must be > 0"))
            } else {
                Ok(value)
            }
        })
}
