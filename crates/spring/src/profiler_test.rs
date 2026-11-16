use std::path::PathBuf;

use fg_extension::{ArchiveKind, ProfileEvent, ProfileRequest};

use super::*;

fn request(event: ProfileEvent, pid: u32, secs: u32) -> ProfileRequest {
    ProfileRequest {
        pid,
        event,
        duration_secs: secs,
        output_path: PathBuf::from("/tmp/profile.collapsed"),
    }
}

#[test]
fn the_command_matches_async_profilers_documented_attach_invocation() {
    let launcher = PathBuf::from("/opt/async-profiler/bin/asprof");
    let command = command(ASYNC_PROFILER, &launcher, &request(ProfileEvent::Cpu, 12345, 30)).unwrap();
    assert_eq!(command.program, launcher);
    assert_eq!(command.args, ["-e", "cpu", "-d", "30", "-o", "collapsed", "-f", "/tmp/profile.collapsed", "12345"]);
}

#[test]
fn the_selected_event_is_passed_by_its_async_profiler_name() {
    let launcher = PathBuf::from("asprof");
    for (event, name) in [(ProfileEvent::Alloc, "alloc"), (ProfileEvent::Lock, "lock"), (ProfileEvent::Wall, "wall")] {
        let command = command(ASYNC_PROFILER, &launcher, &request(event, 7, 5)).unwrap();
        assert_eq!(&command.args[0..2], ["-e", name]);
    }
}

#[test]
fn another_profilers_id_is_not_answered() {
    assert_eq!(command("perf", &PathBuf::from("asprof"), &request(ProfileEvent::Cpu, 1, 1)), None);
}

#[test]
fn the_install_pins_a_checksum_and_names_the_launcher_inside_the_archives_directory() {
    let install = install_for("linux-x64", ArchiveKind::TarGz, "ab");
    assert_eq!(install.download_url, "https://github.com/async-profiler/async-profiler/releases/download/v4.5/async-profiler-4.5-linux-x64.tar.gz");
    assert_eq!((install.dir_name.as_str(), install.launcher.as_str()), ("async-profiler-4.5-linux-x64", "bin/asprof"));
    assert!(install_for("macos", ArchiveKind::Zip, "ab").download_url.ends_with("-macos.zip"));
    if let Some((_, _, sha)) = platform_asset() {
        assert_eq!(sha.len(), 64);
    }
}

#[test]
fn the_profiler_is_contributed_with_an_install_only_where_a_build_exists() {
    let contributions = contributions();
    assert_eq!(contributions[0].id, ASYNC_PROFILER);
    assert_eq!(contributions[0].install.is_some(), platform_asset().is_some());
}
