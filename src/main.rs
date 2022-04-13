use clap::{crate_version, Parser};

mod report;
mod searchopts;

use crate::searchopts::SearchOptions;
use pestr::{Geometry, Reservation};

const DEFAULT_CPUS_PER_NODE: u32 = 128;

#[derive(Parser, Debug)]
#[clap(version = crate_version!(), author = "Andrew Dawson <andrew.dawson@ecmwf.int>")]
#[clap(about = "A PEs and threads calculator")]
struct Args {
    /// number of CPUs per node on the target machine
    #[clap(short = 'n', long, parse(try_from_str=positive_int), default_value_t = DEFAULT_CPUS_PER_NODE)]
    cpus_per_node: u32,

    /// assume hyperthreading (doubles the effective CPUs per node)
    #[clap(short = 'y', long)]
    hyperthreading: bool,

    /// suggest alternative geometries that fill whole nodes
    #[clap(short, long)]
    search: Option<Option<String>>,

    /// output in JSON format
    #[clap(short, long)]
    json_output: bool,

    /// number of PEs (MPI tasks) allocated to the job
    #[clap(parse(try_from_str=positive_int))]
    pes: u32,

    /// number of threads allocated to the job
    #[clap(parse(try_from_str=positive_int))]
    threads: u32,
}

fn main() -> Result<(), String> {
    let opts: Args = Args::parse();

    // Construct the Geometry representing the user's job, and compute its reservation.
    let geom = Geometry::new(
        opts.cpus_per_node,
        opts.hyperthreading,
        opts.pes,
        opts.threads,
    )
    .map_err(|e| format!("{}", e))?;

    let res = Reservation::from_geometry(geom);

    // Determine alternate geometries that yield a full reservation, within the
    // specified parameters. Use an empty list if the user didn't ask for
    // alternate geometries.
    let alternates = match opts.search {
        None => Vec::new(),
        Some(search_option_str) => {
            let search_options = match search_option_str {
                None => SearchOptions::default(),
                Some(s) => SearchOptions::parse(&s)?,
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

    if opts.json_output {
        report::json_reporter(geom, res, alternates);
    } else {
        report::text_reporter(res, alternates)
    }
    Ok(())
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

// The default value for CPUs per node is hard-coded, but can be overridden by
// an environment variable named "PESTR_CORES_PER_NODE". This function provides
// the correct value to the program.
// fn default_cpus_per_node() -> u32 {
//     let hardcoded_default = 128;
//     match env::var("PESTR_CORES_PER_NODE") {
//         Ok(val) => match val.parse::<u32>() {
//             Ok(val) => val,
//             Err(_) => hardcoded_default,
//         },
//         Err(_) => hardcoded_default,
//     }
// }
