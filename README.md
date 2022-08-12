# pestr: PEs and threads calculator

This is a tool for understanding distributed HPC job geometries. You give it a
number of PEs (MPI tasks) and the number of threads allocated to each PE and it
will calculate how many nodes are used, and inform you if the geometry does not
fill the nodes it will be allocated (i.e. if there will be spare CPUs not being
used).

It also features a search option to find alternative job geometries that fill
their whole node reservation (i.e. no idle CPUs) within a tunable set of
parameters.

## Usage

To see how what resources are required for a particular job geometry of 512
PEs each with 16 threads:

    $ pestr 512 16
    64 nodes (8192 CPU cores)

This is using the default number of physical CPUs per node, which in this case
is 128, but can be changed by a command line option or by a configuration file
(see [Configuration](/README.md#Configuration)), for example on a machine with
64 physical CPUs per node:

    $ pestr -n 64 512 16
    128 nodes (8192 CPU cores)

If you provide a job geometry that will leave some CPUs idle pestr will let you
know how many CPUs are idle and where they are:

    $ pestr 128 12
    13 nodes (1664 CPU cores)
    warning: reservation is not filled
      1536 CPU cores in use
      128 CPU cores idle across 13 nodes

If you'd like pestr to suggest a better geometry you can use the search
feature:

    $ pestr 128 12 -s
    13 nodes (1664 CPU cores)
    warning: reservation is not filled
      1536 CPU cores in use
      128 CPU cores idle across 13 nodes
    alternate geometries that fill the reservation:
      96 x 8 (6 nodes; 768 CPU cores)
      112 x 8 (7 nodes; 896 CPU cores)
      128 x 8 (8 nodes; 1024 CPU cores)
      144 x 8 (9 nodes; 1152 CPU cores)
      160 x 8 (10 nodes; 1280 CPU cores)
      96 x 16 (12 nodes; 1536 CPU cores)
      104 x 16 (13 nodes; 1664 CPU cores)
      112 x 16 (14 nodes; 1792 CPU cores)
      120 x 16 (15 nodes; 1920 CPU cores)
      128 x 16 (16 nodes; 2048 CPU cores)
      136 x 16 (17 nodes; 2176 CPU cores)
      144 x 16 (18 nodes; 2304 CPU cores)
      152 x 16 (19 nodes; 2432 CPU cores)
      160 x 16 (20 nodes; 2560 CPU cores)

The search space can be restricted using search options on the command line (or
in a configuration file, see [Configuration](/README.md#Configuration)). You
can independently tune the search radius for PEs and threads, given as a
fraction of the given value, and optionally restrict results to the same number
of nodes as the input geometry. This is useful for fine-tuning geometries to
have them make the best use of their resource allocation. Take the above
geometry as an example, let's restrict the search to filling the same number of
nodes:

    $ pestr 128 12 -s conserve_nodes
    13 nodes (1664 CPU cores)
    warning: reservation is not filled
      1536 CPU cores in use
      128 CPU cores idle across 13 nodes
    alternate geometries that fill the reservation:
      104 x 16 (13 nodes; 1664 CPU cores)

Now we only get one suggestion. Let's now broaden our search radius, whilst
still restricting to the same number of nodes:

    $ pestr 128 12 -s pe_radius=1,thread_radius=2,conserve_nodes
    13 nodes (1664 CPU cores)
    warning: reservation is not filled
      1536 CPU cores in use
      128 CPU cores idle across 13 nodes
    alternate geometries that fill the reservation:
      52 x 32 (13 nodes; 1664 CPU cores)
      104 x 16 (13 nodes; 1664 CPU cores)
      208 x 8 (13 nodes; 1664 CPU cores)

All options are documented with `pestr --help`.


## Configuration

A configuration file can be used to define default parameters. All of the keys
are optional, you can include as many or as few as you like. The following is
an example configuration file:

    # Define the number of physical CPUs per node on your target
    # architecture, this can be overridden by the --cpus-per-node
    # command line option.
    cpus_per_node = 64

    # Options for searching are given inside a [search] section
    
    [search]

    # Parameters tuning the search space when searching for alternate
    # job geometries that fill whole nodes, given as a fraction of the
    # input value.
    pe_radius = 1
    thread_radius = 0.5

    # A boolean indicating if search results must use the same number
    # of nodes as the input geometry.
    conserve_nodes = false

By default pestr will look for a config file in `~/.pestr.toml`, but this can
be overridden by the `--config-file` command line argument.
