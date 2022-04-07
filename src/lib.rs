use serde::Serialize;
use std::fmt;

// ---------------------------------------------------------------------------
// Error handling for bad geometry sizes.
pub struct GeometryError {
    message: String,
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid geometry, {}", self.message)
    }
}

impl fmt::Debug for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GeometryError {{ message: {} }}", self.message)
    }
}

/// A job geometry represents the shape of a job (tasks x threads) and the
/// shape of the resource it is run on.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Geometry {
    cpus_per_node: u32,
    hyperthreading: bool,
    logical_cpus: u32,
    /// The number of MPI tasks (PEs in Cray language) the job uses.
    pub tasks: u32,
    /// The number of threads allocated to each MPI task.
    pub threads: u32,
}

impl Geometry {
    /// Returns a geometry wrapped in a result for the given job specification,
    /// or an error if the geometry is invalid.
    ///
    /// Invalid geometries are those with `cpus_per_node`, `tasks` or `threads less
    /// than 1, and those where `threads` is greater than the number of logical CPUS
    /// available on a node.
    ///
    /// # Arguments
    ///
    /// * `cpus_per_node` - The number of physical CPU cores available per node.
    /// * `hyperthreading` - Whether or not hyperthreading is active, if it is the
    ///                      number of logical CPUs will be double the number given
    ///                      by `cpus_per_node`.
    /// * `tasks` - The number of MPI tasks (PEs) the job is allocated.
    /// * `threads` - The number of threads each MPI task is allocated.
    ///
    /// # Example
    /// ```
    /// use pestr::Geometry;
    /// let geom = match Geometry::new(36, true, 24, 4) {
    ///     Ok(geom) => geom
    ///     Err(e) =};
    /// ```
    pub fn new(
        cpus_per_node: u32,
        hyperthreading: bool,
        tasks: u32,
        threads: u32,
    ) -> Result<Geometry, GeometryError> {
        let logical_cpus = cpus_per_node * if hyperthreading { 2 } else { 1 };
        if cpus_per_node == 0 {
            Err(GeometryError {
                message: String::from("CPUs per node must be > 0"),
            })
        } else if tasks == 0 || threads == 0 {
            Err(GeometryError {
                message: String::from("tasks and threads must be > 0"),
            })
        } else if threads > logical_cpus {
            Err(GeometryError {
                message: String::from("threads cannot be larger than the number of CPUs per node"),
            })
        } else {
            Ok(Geometry {
                cpus_per_node,
                hyperthreading,
                logical_cpus,
                tasks,
                threads,
            })
        }
    }

    /// For a given geometry produce alternate geometries along with their
    /// reservations, that are within a particular size similarity threshold
    /// and fill their whole reservation.
    ///
    /// # Arguments
    ///
    /// * `task_radius` - The search distance for task count expressed as a fraction
    ///                   of the geometry's task count. For example, a value of `0.5`
    ///                   allows alternate geometries with up to 50% more or fewer
    ///                   tasks than this one.
    /// * `thread_radius` - The search distance for thread count expressed as a fraction
    ///                     of the geometry's thread count.
    /// * `filter` - A filter function accepting a geometry and a reservation as inputs
    ///              that returns `true` if the geometry should be used, or `false` if
    ///              it should be ignored. This can be used to restrict the alternates
    ///              to a subset, for example it can be used to select only geometries
    ///              that have the same size reservation as this one.
    ///
    /// # Examples
    ///
    /// Suggest all alternates with 12-36 tasks and 2-6 threads:
    /// ```
    /// use pestr::Geometry;
    /// let geom = Geometry::new(36, false, 24, 4).unwrap();
    /// let alternates = geom.alternates(0.25, 0.5, &|_, _| true);
    /// ```
    ///
    /// Suggest only alternates that have the same size reservation as the current one:
    /// ```
    /// use pestr::{Geometry, Reservation};
    /// let geom = Geometry::new(36, false, 120, 6).unwrap();
    /// let res = Reservation::from_geometry(geom);
    /// let alternates = geom.alternates(0.25, 0.5, &|_, r| { r.nodes == res.nodes });
    /// ```
    pub fn alternates(
        self,
        task_radius: f32,
        thread_radius: f32,
        filter: &dyn Fn(Geometry, Reservation) -> bool,
    ) -> Vec<(Geometry, Reservation)> {
        let task_delta = (task_radius * (self.tasks as f32)) as i64;
        let thread_delta = (thread_radius * (self.threads as f32)) as i64;
        let mut alternates = Vec::new();
        for task_p in -task_delta..=task_delta {
            let tasks = ((self.tasks as i64) + task_p) as u32;
            if tasks < 1 {
                continue;
            }
            for thread_p in -thread_delta..=thread_delta {
                let threads = ((self.threads as i64) + thread_p) as u32;
                if threads < 1 || threads > self.logical_cpus {
                    continue;
                }
                let geom = Geometry::with_tasks_and_threads(self, tasks, threads);
                let res = Reservation::from_geometry(geom);
                if res.is_filled && filter(geom, res) {
                    alternates.push((geom, res));
                }
            }
        }
        alternates.sort_by(|(_, a), (_, b)| a.nodes.cmp(&b.nodes));
        alternates
    }

    fn with_tasks_and_threads(geom: Geometry, tasks: u32, threads: u32) -> Geometry {
        Geometry::new(geom.cpus_per_node, geom.hyperthreading, tasks, threads).unwrap()
    }
}

/// A reservation represents the resources required to run a job of a particular geometry.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Reservation {
    /// The total number of nodes in the reservation.
    pub nodes: u32,
    /// The total number of logical CPUs in the reservation.
    pub cpus: u32,
    is_filled: bool,
    /// The number of logical CPUs that are active.
    pub used_cpus: u32,
    /// The number of logical CPUs that are reserved but idle.
    pub idle_cpus: u32,
    /// The number of nodes in the reservation that have 1 or more idle CPUs in them.
    pub partial_nodes: u32,
}

impl Reservation {
    /// Create a reservation from a geometry.
    ///
    /// # Arguments
    ///
    /// * geom - A geometry to construct a reservation from.
    ///
    /// # Examples
    /// ```
    /// use pestr::{Geometry, Reservation};
    /// let geom = Geometry::new(36, false, 24, 4).unwrap();
    /// let res = Reservation::from_geometry(geom);
    /// ```
    pub fn from_geometry(geom: Geometry) -> Reservation {
        fn compute_cpu_list(geom: Geometry) -> Vec<u32> {
            let tasks_per_node = geom.logical_cpus / geom.threads;
            let num_nodes = geom.tasks / tasks_per_node;
            let remainder = geom.tasks - (num_nodes * tasks_per_node);
            let mut nodes = vec![tasks_per_node * geom.threads; num_nodes as usize];
            if remainder > 0 {
                nodes.push(remainder * geom.threads);
            }
            nodes
        }
        let cpu_list = compute_cpu_list(geom);
        let reserved_nodes: u32 = cpu_list.len() as u32;
        let reserved_cpus = reserved_nodes * geom.logical_cpus;
        let used_cpus = cpu_list.iter().sum();
        if reserved_cpus == used_cpus {
            Reservation {
                nodes: reserved_nodes,
                cpus: reserved_cpus,
                is_filled: true,
                used_cpus: reserved_cpus,
                idle_cpus: 0,
                partial_nodes: 0,
            }
        } else {
            let partial_nodes = cpu_list.iter().filter(|&n| *n < geom.logical_cpus).count() as u32;
            Reservation {
                nodes: reserved_nodes,
                cpus: reserved_cpus,
                is_filled: false,
                used_cpus,
                idle_cpus: reserved_cpus - used_cpus,
                partial_nodes,
            }
        }
    }
}
