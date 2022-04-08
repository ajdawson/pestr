use clap::{crate_version, Parser};
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::json;

use pestr::{Geometry, Reservation};

const DEFAULT_SEARCH_CONSERVE_NODES: bool = false;
const DEFAULT_SEARCH_PE_RADIUS: f32 = 0.25;
const DEFAULT_SEARCH_THREAD_RADIUS: f32 = 0.5;

#[derive(Parser, Debug)]
#[clap(version = crate_version!(), author = "Andrew Dawson <andrew.dawson@ecmwf.int>")]
#[clap(about = "A PEs and threads calculator")]
struct Args {
    /// number of CPUs per node on the target machine
    #[clap(short = 'n', long, parse(try_from_str=positive_int), default_value_t = 128)]
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

#[derive(Debug)]
struct SearchOptions {
    conserve_nodes: bool,
    pe_radius: f32,
    thread_radius: f32,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            conserve_nodes: DEFAULT_SEARCH_CONSERVE_NODES,
            pe_radius: DEFAULT_SEARCH_PE_RADIUS,
            thread_radius: DEFAULT_SEARCH_THREAD_RADIUS,
        }
    }
}

impl SearchOptions {
    fn parse(s: &str) -> Result<Self, String> {
        lazy_static! {
            static ref PE_RADIUS_REGEX: Regex =
                Regex::new(r#"^pe_radius=(?P<value>[0-9]*\.?[0-9]+)$"#).unwrap();
            static ref THREAD_RADIUS_REGEX: Regex =
                Regex::new(r#"^thread_radius=(?P<value>[0-9]*\.?[0-9]+)$"#).unwrap();
        }

        let mut conserve_nodes = DEFAULT_SEARCH_CONSERVE_NODES;
        let mut pe_radius = DEFAULT_SEARCH_PE_RADIUS;
        let mut thread_radius = DEFAULT_SEARCH_THREAD_RADIUS;

        for opt in s.split(',') {
            if opt == "conserve_nodes" {
                conserve_nodes = true;
            } else if PE_RADIUS_REGEX.is_match(opt) {
                let caps = PE_RADIUS_REGEX.captures(opt).unwrap();
                let m = caps.name("value").unwrap();
                pe_radius = m.as_str().parse().unwrap();
            } else if THREAD_RADIUS_REGEX.is_match(opt) {
                let caps = THREAD_RADIUS_REGEX.captures(opt).unwrap();
                let m = caps.name("value").unwrap();
                thread_radius = m.as_str().parse().unwrap();
            } else {
                return Err(format!("unknown search option: {}", opt));
            }
        }

        Ok(Self {
            conserve_nodes,
            pe_radius,
            thread_radius,
        })
    }
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
        json_reporter(geom, res, alternates);
    } else {
        text_reporter(res, alternates)
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Functions for validating command line inputs.
//

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

// ---------------------------------------------------------------------------
// Reporter functions that report node usage/suggestions in a particular
// format.
//

// Reporting in JSON format
fn json_reporter(geom: Geometry, res: Reservation, alternates: Vec<(Geometry, Reservation)>) {
    fn jsonize_job(geom: Geometry, res: Reservation) -> serde_json::Value {
        json!({"geometry": geom, "reservation": res})
    }
    let report = json!({
        "geometry": geom,
        "reservation": res,
        "alternatives": alternates
                        .iter()
                        .map(|&(g, r)| jsonize_job(g, r))
                        .collect::<Vec<serde_json::Value>>(),
    });
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}

// Reporting in human-readable plain text
fn text_reporter(res: Reservation, alternates: Vec<(Geometry, Reservation)>) {
    fn print_reservation(res: Reservation) {
        println!("{} nodes ({} CPU cores)", res.nodes, res.cpus);
        if res.used_cpus != res.cpus {
            println!("warning: reservation is not filled");
            println!("  {} CPU cores in use", res.used_cpus);
            println!(
                "  {} CPU cores idle across {} nodes",
                res.idle_cpus, res.partial_nodes
            );
        }
    }

    fn print_job(geom: Geometry, res: Reservation) {
        println!(
            "  {} x {} ({} nodes; {} CPU cores)",
            geom.tasks, geom.threads, res.nodes, res.cpus
        );
    }

    print_reservation(res);
    if !alternates.is_empty() {
        println!("alternate geometries that fill the reservation:");
        for (g, r) in alternates {
            print_job(g, r);
        }
    }
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
