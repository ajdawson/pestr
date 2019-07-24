#[macro_use]
extern crate clap;
use std::env;
use clap::{App, Arg};
use pestr::{better_geometries, make_res, Geometry, Reservation};
use serde_json::{json, to_string_pretty};

macro_rules! get_arg {
    ($m:ident.value_of($v:expr), $t:ty) => {
        get_arg!($m, $v, $t)
    };
    ($m:ident, $v:expr, $t:ty) => {
        value_t!($m, $v, $t).unwrap_or_else(|e| e.exit())
    };
}

fn positive_int_validator(val: String) -> Result<(), String> {
    match val.parse::<u32>() {
        Ok(x) => match x {
            0 => Err(String::from("must be > 0")),
            _ => Ok(()),
        },
        Err(..) => Err(String::from("must be a positive integer")),
    }
}

fn float_validator(val: String) -> Result<(), String> {
    match val.parse::<f32>() {
        Ok(..) => Ok(()),
        Err(..) => Err(String::from("must be a real number")),
    }
}

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

fn json_reporter(geom: Geometry, res: Reservation, alternates: Vec<(Geometry, Reservation)>) {
    fn jsonize_job(geom: Geometry, res: Reservation) -> serde_json::Value {
        json!({
            "pes": geom.tasks,
            "threads": geom.threads,
            "nodes": res.nodes,
            "cpus": res.cpus,
        })
    }
    let a_json: Vec<serde_json::Value> = alternates
        .iter()
        .map(|&(g, r)| jsonize_job(g, r))
        .collect();
    let report = json!({
        "cpus_per_node": geom.cpus_per_node,
        "hyperthreading": geom.hyperthreading,
        "pes": geom.tasks,
        "threads": geom.threads,
        "full_reservation": res.used_cpus == res.cpus,
        "nodes": res.nodes,
        "cpus": res.cpus,
        "used_cpus": res.used_cpus,
        "idle_cpus": res.idle_cpus,
        "partial_nodes": res.partial_nodes,
        "alternatives": a_json,
    });
    println!("{}", to_string_pretty(&report).unwrap());
}

fn text_reporter(res: Reservation, alternates: Vec<(Geometry, Reservation)>) {
    print_reservation(res);
    if alternates.len() > 0 {
        println!("alternate geometries that fill the reservation:");
        for (g, r) in alternates {
            print_job(g, r);
        }
    }
}

fn default_cpus_per_node() -> u32 {
    let hardcoded_default = 36;
    match env::var("PESTR_CORES_PER_NODE") {
        Ok(val) => match val.parse::<u32>() {
            Ok(val) => val,
            Err(_) => hardcoded_default,
        }
        Err(_) => hardcoded_default
    }
}

fn main() {
    let default_cpus_per_node_str: &str = &default_cpus_per_node().to_string();
    let app = App::new("pestr")
        .version("1.0")
        .arg(
            Arg::with_name("PES")
                .help("Number of PEs (MPI tasks) allocated to the job")
                .validator(positive_int_validator)
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("THREADS")
                .help("Number of threads per PE")
                .validator(positive_int_validator)
                .required(true)
                .index(2),
        )
        .arg(
            Arg::with_name("cpus_per_node")
                .short("n")
                .help("Number of CPUs per node on the target machine")
                .validator(positive_int_validator)
                .default_value(default_cpus_per_node_str)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("hyperthreading")
                .short("y")
                .help("Enable hyperthreading (double the node CPU count)"),
        )
        .arg(
            Arg::with_name("suggest_alternates")
                .short("s")
                .help("Suggest alternate geometries that fill their reservation"),
        )
        .arg(
            Arg::with_name("conserve_node_count")
                .short("c")
                .help("Conserve total node count in suggested geometries"),
        )
        .arg(
            Arg::with_name("pe_radius")
                .short("p")
                .help("blah")
                .validator(float_validator)
                .default_value("0.25")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("thread_radius")
                .short("t")
                .help("blah")
                .validator(float_validator)
                .default_value("0.5")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("json_output")
                .short("j")
                .help("Write output as JSON"),
        );
    let matches = app.get_matches();

    let get_flag = |name| -> bool {
        if matches.is_present(name) {
            true
        } else {
            false
        }
    };

    
    let geom = Geometry {
        cpus_per_node: get_arg!(matches.value_of("cpus_per_node"), u32),
        hyperthreading: get_flag("hyperthreading"),
        tasks: get_arg!(matches.value_of("PES"), u32),
        threads: get_arg!(matches.value_of("THREADS"), u32),
    };
    let res = make_res(geom);

    let alternates = if res.cpus != res.used_cpus && get_flag("suggest_alternates") {
        let task_radius = get_arg!(matches.value_of("pe_radius"), f32);
        let thread_radius = get_arg!(matches.value_of("thread_radius"), f32);
        let gr_filter = |_, r: Reservation| -> bool {
            if get_flag("conserve_node_count") {
                r.nodes == res.nodes
            } else {
                true
            }
        };
        better_geometries(task_radius, thread_radius, &gr_filter, geom)
    } else {
        Vec::new()
    };

    if get_flag("json_output") {
        json_reporter(geom, res, alternates);
    } else {
        text_reporter(res, alternates)
    }
}
