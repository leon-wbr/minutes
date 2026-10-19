//! Isolated CPU inference only. Never captures, downloads, enrolls or writes audio.
#[allow(dead_code)]
mod protocol;
#[cfg(windows)]
mod windows_handles;
use protocol::*;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Duration;
use zeroize::Zeroizing;

fn main() {
    if let Err(error) = run() {
        eprintln!("full speaker worker: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    windows_handles::close_inherited_handles()?;
    if std::env::var("MINUTES_FULL_SPEAKER_WORKER_V1").as_deref() != Ok("1") {
        return Err("launch through the Minutes supervisor".into());
    }
    install_limits()?;
    #[cfg(unix)]
    let parent: u32 = std::env::var("MINUTES_FULL_SPEAKER_PARENT_PID")?.parse()?;
    #[cfg(unix)]
    if parent <= 1 || unsafe { libc::getppid() } as u32 != parent {
        return Err("supervising parent is unavailable".into());
    }
    std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(900);
        loop {
            #[cfg(unix)]
            if unsafe { libc::getppid() } as u32 != parent {
                std::process::exit(70);
            }
            if std::time::Instant::now() >= deadline {
                std::process::exit(70);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
    let mut args = std::env::args_os().skip(1);
    let models = args
        .next()
        .map(PathBuf::from)
        .ok_or("models directory required")?;
    if args.next().is_some() {
        return Err("unexpected arguments".into());
    }
    // Hash the private copy that inference will reopen, rather than trusting
    // a model pathname that can change between verification and loading.
    let snapshot = tempfile::tempdir()?;
    for (name, _) in MODEL_FILES {
        let mut input = std::fs::File::open(models.join(name))?.take(40 * 1024 * 1024 + 1);
        let mut output = std::fs::File::create(snapshot.path().join(name))?;
        std::io::copy(&mut input, &mut output)?;
    }
    verify_models(snapshot.path())?;
    let mut samples = Zeroizing::new(Vec::new());
    let mut input = std::io::stdin().lock();
    let mut hasher = Sha256::new();
    loop {
        let mut bytes = [0; 4];
        let first = input.read(&mut bytes[..1])?;
        if first == 0 {
            break;
        }
        input.read_exact(&mut bytes[1..])?;
        if samples.len() >= MAX_SAMPLES {
            return Err("one-hour audio budget exceeded".into());
        }
        let value = f32::from_le_bytes(bytes);
        if !value.is_finite() {
            return Err("non-finite PCM input".into());
        }
        hasher.update(bytes);
        samples.push(value);
    }
    if samples.is_empty() {
        return Err("empty PCM input".into());
    }
    let sample_count = samples.len();
    let audio_sha256 = format!("{:x}", hasher.finalize());
    let mut pipeline =
        speakrs::OwnedDiarizationPipeline::from_dir(snapshot.path(), speakrs::ExecutionMode::Cpu)?;
    let step = pipeline.segmentation_step();
    if !step.is_finite()
        || step < 1.0
        || ((samples.len() as f64 / 16_000.0 / step).ceil() + 10.0) * 3.0 > 12_000.0
    {
        return Err("full speaker observation budget exceeded".into());
    }
    let result = pipeline.run(&samples)?;
    if result.segments.len() > MAX_SEGMENTS {
        return Err("segment budget exceeded".into());
    }
    let duration = sample_count as f64 / 16_000.0;
    let mut segments = Vec::new();
    for segment in result.segments {
        if let Some(segment) =
            canonical_segment(&segment.speaker, segment.start, segment.end, duration)?
        {
            segments.push(segment);
        }
    }
    let output = serde_json::to_vec(&Response {
        protocol: PROTOCOL,
        engine: ENGINE.into(),
        model_revision: MODEL_REVISION.into(),
        sample_count,
        audio_sha256,
        segments,
    })?;
    if output.len() as u64 > OUTPUT_LIMIT {
        return Err("response byte budget exceeded".into());
    }
    std::io::stdout().lock().write_all(&output)?;
    Ok(())
}

fn canonical_segment(
    speaker: &str,
    start: f64,
    end: f64,
    duration: f64,
) -> Result<Option<Segment>, Box<dyn std::error::Error>> {
    let index: u8 = speaker
        .strip_prefix("SPEAKER_")
        .ok_or("unexpected native speaker label")?
        .parse()?;
    if index >= 64 || speaker != format!("SPEAKER_{index:02}") {
        return Err("unexpected native speaker label or speaker budget exceeded".into());
    }
    // Validate before clamping: f64::min would otherwise hide a NaN end time.
    if !start.is_finite() || !end.is_finite() || start < 0.0 || end <= start {
        return Err("invalid native speaker activity".into());
    }
    let end = end.min(duration);
    Ok((start < end).then(|| Segment {
        speaker: format!("SPEAKER_{}", index + 1),
        start,
        end,
    }))
}

#[cfg(unix)]
fn install_limits() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    let limit = {
        use mach2::kern_return::KERN_SUCCESS;
        use mach2::task::task_info;
        use mach2::task_info::{
            task_basic_info_64, task_info_t, TASK_BASIC_INFO_64, TASK_BASIC_INFO_64_COUNT,
        };
        let mut info = task_basic_info_64::default();
        let mut count = TASK_BASIC_INFO_64_COUNT;
        let status = unsafe {
            task_info(
                mach2::traps::mach_task_self(),
                TASK_BASIC_INFO_64,
                (&mut info as *mut task_basic_info_64).cast::<libc::c_int>() as task_info_t,
                &mut count,
            )
        };
        if status != KERN_SUCCESS || count != TASK_BASIC_INFO_64_COUNT {
            return Err("cannot measure worker virtual memory".into());
        }
        info.virtual_size
            .checked_add(MEMORY_BYTES)
            .ok_or("memory budget overflow")?
    };
    #[cfg(not(target_os = "macos"))]
    let limit = MEMORY_BYTES;
    let memory = libc::rlimit {
        rlim_cur: limit as libc::rlim_t,
        rlim_max: limit as libc::rlim_t,
    };
    if unsafe { libc::setrlimit(libc::RLIMIT_AS, &memory) } != 0 {
        return Err("cannot install worker memory ceiling".into());
    }
    unsafe {
        libc::setpriority(libc::PRIO_PROCESS, 0, 10);
    }
    Ok(())
}

#[cfg(windows)]
fn install_limits() -> Result<(), Box<dyn std::error::Error>> {
    // Only the host can attach the suspended child before its first instruction.
    // Refuse standalone use rather than imply a Windows memory fence exists.
    use windows_sys::Win32::System::JobObjects::{
        IsProcessInJob, JobObjectExtendedLimitInformation, QueryInformationJobObject,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_JOB_MEMORY,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_PROCESS_MEMORY,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    let mut in_job = 0;
    if unsafe { IsProcessInJob(GetCurrentProcess(), std::ptr::null_mut(), &mut in_job) } == 0
        || in_job == 0
    {
        return Err("Windows speaker worker requires a supervising Job Object".into());
    }
    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe {
        QueryInformationJobObject(
            std::ptr::null_mut(),
            JobObjectExtendedLimitInformation,
            (&mut info as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            std::mem::size_of_val(&info) as u32,
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err("cannot inspect supervising Job Object".into());
    }
    let flags = JOB_OBJECT_LIMIT_PROCESS_MEMORY
        | JOB_OBJECT_LIMIT_JOB_MEMORY
        | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    if info.BasicLimitInformation.LimitFlags & flags != flags
        || info.ProcessMemoryLimit != MEMORY_BYTES as usize
        || info.JobMemoryLimit != MEMORY_BYTES as usize
    {
        return Err("Windows speaker worker memory ceiling is missing".into());
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn install_limits() -> Result<(), Box<dyn std::error::Error>> {
    Err("unsupported worker platform".into())
}

#[cfg(test)]
mod tests {
    #[test]
    fn invalid_native_activity_cannot_be_hidden_by_padding_clamp() {
        for (start, end) in [
            (0.0, f64::NAN),
            (f64::NAN, 1.0),
            (0.0, f64::INFINITY),
            (-1.0, 1.0),
            (1.0, 1.0),
            (2.0, 1.0),
        ] {
            assert!(super::canonical_segment("SPEAKER_00", start, end, 2.0).is_err());
        }
        for label in ["Mat", "SPEAKER_0", "SPEAKER_64"] {
            assert!(super::canonical_segment(label, 0.0, 1.0, 2.0).is_err());
        }
        let padded = super::canonical_segment("SPEAKER_00", 1.0, 3.0, 2.0)
            .unwrap()
            .unwrap();
        assert_eq!(
            (padded.speaker.as_str(), padded.start, padded.end),
            ("SPEAKER_1", 1.0, 2.0)
        );
        assert!(super::canonical_segment("SPEAKER_00", 2.0, 3.0, 2.0)
            .unwrap()
            .is_none());
    }

    #[cfg(unix)]
    #[test]
    fn memory_ceiling_refuses_an_over_budget_mapping() {
        const MARKER: &str = "MINUTES_SPEAKER_MEMORY_TEST_CHILD";
        if std::env::var_os(MARKER).is_some() {
            super::install_limits().unwrap();
            let bytes = (super::MEMORY_BYTES * 2) as usize;
            let mapping = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    bytes,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_PRIVATE | libc::MAP_ANON,
                    -1,
                    0,
                )
            };
            if mapping != libc::MAP_FAILED {
                unsafe {
                    libc::munmap(mapping, bytes);
                }
                panic!("worker memory ceiling allowed an over-budget mapping");
            }
            return;
        }
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "tests::memory_ceiling_refuses_an_over_budget_mapping",
                "--nocapture",
            ])
            .env(MARKER, "1")
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "memory ceiling child failed");
                break;
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("memory ceiling child did not finish");
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}
