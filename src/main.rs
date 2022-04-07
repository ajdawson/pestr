use clap::{crate_version, Parser};
use serde_json::json;
use std::env;

use pestr::{Geometry, GeometryError, Reservation};

// ---------------------------------------------------------------------------
// Functions for validating command line inputs.
//

fn positive_int(s: &str) -> Result<u32, String> {
    let value: u32 = s
        .parse()
        .map_err(|_| format!("must be a positive integer"))?;
    
    if value == 0 {
        Err(format!("must be a positive integer"))
    } else {
        Ok(value)
    }
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
fn default_cpus_per_node() -> u32 {
    let hardcoded_default = 128;
    match env::var("PESTR_CORES_PER_NODE") {
        Ok(val) => match val.parse::<u32>() {
            Ok(val) => val,
            Err(_) => hardcoded_default,
        },
        Err(_) => hardcoded_default,
    }
}

#[derive(Parser, Debug)]
#[clap(version = crate_version!(), author = "Andrew Dawson <andrew.dawson@ecmwf.int>")]
#[clap(about = "A PEs and threads calculator")]
struct Args {
    /// number of CPUs per node on the target machine
    #[clap(short = 'n', long, parse(try_from_str=positive_int), default_value = "128")]
    cpus_per_node: u32,

    /// assume hyperthreading (doubles the effective CPUs per node)
    #[clap(short = 'y', long)]
    hyperthreading: bool,

    /// suggest alternative geometries that fill whole nodes
    #[clap(short, long)]
    suggest: bool,

    /// suggestions should conserve the total number of nodes used
    #[clap(short, long)]
    conserve_node_count: bool,

    /// blah?
    #[clap(short, long, default_value = "0.5")]
    pe_radius: f32,

    /// huh?
    #[clap(short, long, default_value = "0.25")]
    thread_radius: f32,

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
    ).map_err(|e| format!("{}", e))?;

    let res = Reservation::from_geometry(geom);

    // Determine alternate geometries that yield a full reservation, within the
    // specified parameters. Use an empty list if the user didn't ask for
    // alternate geometries.
    let alternates = if opts.suggest {
        let task_radius = opts.pe_radius;
        let thread_radius = opts.thread_radius;
        let gr_filter = |_, r: Reservation| -> bool {
            if opts.conserve_node_count {
                r.nodes == res.nodes
            } else {
                true
            }
        };
        geom.alternates(task_radius, thread_radius, &gr_filter)
    } else {
        Vec::new()
    };

    if opts.json_output {
        json_reporter(geom, res, alternates);
    } else {
        text_reporter(res, alternates)
    }
    Ok(())
}
