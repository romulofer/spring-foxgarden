//! async-profiler, the JVM sampling profiler (`PLAN.md` Track 26, moved out of
//! the editor by Track 24).
//!
//! The editor shells out to async-profiler's own `asprof` launcher against a
//! pid: `asprof -e <event> -d <secs> -o collapsed -f <file> <pid>` is the
//! project's documented attach invocation, and `-d` is itself a shortcut for
//! start/sleep/stop, so one process both attaches and detaches. `-o collapsed`
//! is the folded-stack format the editor's flame graph reads.
//!
//! async-profiler publishes linux-x64/arm64 tarballs and one macOS zip, and
//! nothing for Windows, so there `install` is `None` and the editor says so.
//! Every version, URL and checksum below was verified against the real v4.5
//! release archives (downloaded, `sha256sum`'d and listed), not assumed from
//! the release page's naming.

use std::path::Path;

use fg_extension::{ArchiveKind, CommandSpec, ProfileEvent, ProfileRequest, ProfilerContribution, ProfilerInstall};

/// The profiler's registered id.
pub const ASYNC_PROFILER: &str = "async-profiler";

/// The pinned release: a specific, verified one, not whatever is newest.
const VERSION: &str = "4.5";

/// The release archive for the platform this build runs on, as `(asset suffix,
/// archive kind, sha256)`. macOS ships one universal zip with no arch in its
/// name, so both Mac arches share it.
fn platform_asset() -> Option<(&'static str, ArchiveKind, &'static str)> {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some(("linux-x64", ArchiveKind::TarGz, "89546fbb9ee0fc5496c7edd4099b0709489bc78b0d8057ccbb4b801f6b032b62"))
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        Some(("linux-arm64", ArchiveKind::TarGz, "64c41d1465d60097439c50d7e924b4946f1f62b1cbd21ce5b034fad09c0d6979"))
    } else if cfg!(target_os = "macos") {
        Some(("macos", ArchiveKind::Zip, "46d04ef81f532a065a0b3877e488aa706afa14aa2ea14433b323db9e6fda76dc"))
    } else {
        None
    }
}

fn install_for(suffix: &str, archive: ArchiveKind, sha256: &str) -> ProfilerInstall {
    let extension = match archive {
        ArchiveKind::Zip => "zip",
        ArchiveKind::TarGz => "tar.gz",
    };
    let dir_name = format!("async-profiler-{VERSION}-{suffix}");
    ProfilerInstall {
        version: VERSION.to_string(),
        download_url: format!("https://github.com/async-profiler/async-profiler/releases/download/v{VERSION}/{dir_name}.{extension}"),
        sha256: sha256.to_string(),
        archive,
        dir_name,
        launcher: "bin/asprof".to_string(),
    }
}

pub fn contributions() -> Vec<ProfilerContribution> {
    vec![ProfilerContribution {
        id: ASYNC_PROFILER.to_string(),
        display_name: "async-profiler".to_string(),
        install: platform_asset().map(|(suffix, archive, sha256)| install_for(suffix, archive, sha256)),
    }]
}

/// The `asprof` invocation that attaches to the request's pid, samples for its
/// duration and writes collapsed stacks to its output path.
pub fn command(profiler_id: &str, launcher: &Path, request: &ProfileRequest) -> Option<CommandSpec> {
    if profiler_id != ASYNC_PROFILER {
        return None;
    }
    let event = match request.event {
        ProfileEvent::Cpu => "cpu",
        ProfileEvent::Alloc => "alloc",
        ProfileEvent::Lock => "lock",
        ProfileEvent::Wall => "wall",
    };
    let cwd = launcher.parent().unwrap_or(Path::new("."));
    Some(
        CommandSpec::new(launcher, cwd)
            .arg("-e")
            .arg(event)
            .arg("-d")
            .arg(request.duration_secs.to_string())
            .arg("-o")
            .arg("collapsed")
            .arg("-f")
            .arg(request.output_path.display().to_string())
            .arg(request.pid.to_string()),
    )
}

#[cfg(test)]
#[path = "profiler_test.rs"]
mod profiler_test;
