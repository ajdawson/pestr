#[derive(Clone, Copy, Debug)]
pub struct Geometry {
    pub cpus_per_node: u32,
    pub hyperthreading: bool,
    pub tasks: u32,
    pub threads: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Reservation {
    pub nodes: u32,
    pub cpus: u32,
    pub used_cpus: u32,
    pub idle_cpus: u32,
    pub partial_nodes: u32,
}

fn is_filled(res: Reservation) -> bool {
    res.used_cpus == res.cpus
}

fn total_cpus(geom: Geometry) -> u32 {
    if geom.hyperthreading {
        geom.cpus_per_node * 2
    } else {
        geom.cpus_per_node
    }
}

fn compute_cpu_list(geom: Geometry) -> Vec<u32> {
    let tasks_per_node = total_cpus(geom) / geom.threads;
    let num_nodes = geom.tasks / tasks_per_node;
    let remainder = geom.tasks - (num_nodes * tasks_per_node);
    let mut nodes = vec![tasks_per_node * geom.threads; num_nodes as usize];
    if remainder > 0 {
        nodes.push(remainder * geom.threads);
    }
    nodes
}

pub fn make_res(geom: Geometry) -> Reservation {
    let cpu_list = compute_cpu_list(geom);
    let reserved_nodes: u32 = cpu_list.len() as u32;
    let reserved_cpus = reserved_nodes * total_cpus(geom);
    let used_cpus = cpu_list.iter().sum();
    if reserved_cpus == used_cpus {
        Reservation {
            nodes: reserved_nodes,
            cpus: reserved_cpus,
            used_cpus: reserved_cpus,
            idle_cpus: 0,
            partial_nodes: 0,
        }
    } else {
        let partial_nodes = cpu_list.iter().filter(|&n| *n < total_cpus(geom)).count() as u32;
        Reservation {
            nodes: reserved_nodes,
            cpus: reserved_cpus,
            used_cpus: used_cpus,
            idle_cpus: reserved_cpus - used_cpus,
            partial_nodes: partial_nodes,
        }
    }
}

pub fn better_geometries(
    task_f: f32,
    thread_f: f32,
    gr_filter: &Fn(Geometry, Reservation) -> bool,
    geom: Geometry,
) -> Vec<(Geometry, Reservation)> {
    let task_delta = (task_f * (geom.tasks as f32)) as i64;
    let thread_delta = (thread_f * (geom.threads as f32)) as i64;
    let mut alternates = Vec::new();
    for task_p in -task_delta..=task_delta {
        let new_tasks = ((geom.tasks as i64) + task_p) as u32;
        if new_tasks < 1 {
            continue;
        }
        for thread_p in -thread_delta..=thread_delta {
            let new_threads = ((geom.threads as i64) + thread_p) as u32;
            if new_threads < 1 || new_threads > total_cpus(geom) {
                continue;
            }
            let new_geom = Geometry {
                cpus_per_node: geom.cpus_per_node,
                hyperthreading: geom.hyperthreading,
                tasks: new_tasks,
                threads: new_threads,
            };
            let res = make_res(new_geom);
            if is_filled(res) && gr_filter(new_geom, res) {
                alternates.push((new_geom, res));
            }
        }
    }
    alternates.sort_by(|(_, a), (_, b)| a.nodes.cmp(&b.nodes));
    return alternates;
}
