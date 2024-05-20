use std::io::Result;

use pb_build::pb_build;

fn main() -> Result<()> {
    let features = vec![];
    let extra_includes = vec![];
    pb_build(features, extra_includes)
}
