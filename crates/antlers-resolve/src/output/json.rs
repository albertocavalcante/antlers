//! JSON output formatter.

use antlers_lock::Lockfile;

use crate::Result;

/// Outputs the lockfile in canonical JSON format.
pub fn format(lockfile: &Lockfile) -> Result<String> {
    Ok(lockfile.write()?)
}
