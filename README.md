# MCT - MPI-Cluster Tools

A collection of cluster management commands for HPC environments, particularly designed for HTCondor-based clusters.

## Installation

```bash
cargo install mpi_cluster_tools
```

## Usage

### Configure Login Credentials

```bash
mct login
```

This command allows you to configure connection settings in two ways:
- **Manual configuration**: Enter hostname, username, and optional identity file
- **SSH config**: Use an existing SSH configuration entry

Configuration is saved to `~/.cluster_tools`.

### Analyze Job Prices

```bash
mct price
```

Connects to your configured cluster and analyzes job pricing based on HTCondor job priorities. Provides breakdowns for:
- GPU vs Non-GPU jobs
- Idle vs Running jobs
- Average prices for each category

## Features

- **Secure SSH connections** using your existing SSH configuration or manual setup
- **Job price analysis** with detailed breakdowns by GPU usage and job status
- **Persistent configuration** stored in your home directory
- **Clean CLI interface** with helpful error messages

## Requirements

- Rust 1.70+ (for building from source)
- SSH access to an HTCondor cluster
- HTCondor `condor_q` command available on the target cluster

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
