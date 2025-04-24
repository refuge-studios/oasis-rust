# oasis-rust
Support for Oasis with Rust.
Go to https://oasis.refugestudios.com.au/ and download the Oasis API.

Split into 3 projects.
- builder
- viewer
- oasis_bindings

Viewer command: `LD_LIBRARY_PATH=lib cargo run -p viewer`
Builder command: `LD_LIBRARY_PATH=lib cargo run -p builder -- <obj_path> <depth> <step levels> [output_name]` step levels disabled currently.