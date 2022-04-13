use serde_json::json;

use pestr::{Geometry, Reservation};

// Reporting in JSON format
pub fn json_reporter(geom: Geometry, res: Reservation, alternates: Vec<(Geometry, Reservation)>) {
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
pub fn text_reporter(res: Reservation, alternates: Vec<(Geometry, Reservation)>) {
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
