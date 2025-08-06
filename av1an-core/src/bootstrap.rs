use crate::context::ResumeOption;
use crate::{create_dir, get_done, init_done, DoneJson};
use anyhow::Context;
use dashmap::DashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::atomic;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use tracing::error;
use tracing::{debug, info};

/// Initialize logging routines and create temporary directories
#[tracing::instrument(level = "debug")]
pub fn prepare_environment(resume_options: &mut ResumeOption) -> anyhow::Result<usize> {
    resume_options.resume = false;

    if !resume_options.resume && Path::new(&resume_options.temp).is_dir() {
        fs::remove_dir_all(&resume_options.temp).with_context(|| {
            format!(
                "Failed to remove temporary directory {temp}",
                temp = resume_options.temp
            )
        })?;
    }

    create_dir!(Path::new(&resume_options.temp))?;
    create_dir!(Path::new(&resume_options.temp).join("split"))?;
    create_dir!(Path::new(&resume_options.temp).join("encode"))?;

    debug!("temporary directory: {temp}", temp = &resume_options.temp);

    let done_path = Path::new(&resume_options.temp).join("done.json");
    let done_json_exists = done_path.exists();
    let chunks_json_exists = Path::new(&resume_options.temp).join("chunks.json").exists();

    if resume_options.resume {
        match (done_json_exists, chunks_json_exists) {
            // both files exist, so there is no problem
            (true, true) => {},
            (false, true) => {
                info!(
                    "resume was set but done.json does not exist in temporary directory {temp}",
                    temp = resume_options.temp
                );
                resume_options.resume = false;
            },
            (true, false) => {
                info!(
                    "resume was set but chunks.json does not exist in temporary directory \
                         {temp}",
                    temp = resume_options.temp
                );
                resume_options.resume = false;
            },
            (false, false) => {
                info!(
                    "resume was set but neither chunks.json nor done.json exist in temporary \
                         directory {temp}",
                    temp = resume_options.temp
                );
                resume_options.resume = false;
            },
        }
    }
    let mut resume_frames: usize = 0;
    if resume_options.resume && done_json_exists {
        let done = fs::read_to_string(done_path)
            .with_context(|| "Failed to read contents of done.json")?;
        let done: DoneJson =
            serde_json::from_str(&done).with_context(|| "Failed to parse done.json")?;
        let done_frames = done.frames.load(atomic::Ordering::Relaxed);

        resume_frames = done_frames;

        init_done(done);
    } else {
        init_done(DoneJson {
            frames: AtomicUsize::new(0),
            done: DashMap::new(),
            audio_done: AtomicBool::new(false),
        });

        let mut done_file = File::create(&done_path)?;
        done_file.write_all(serde_json::to_string(get_done())?.as_bytes())?;
    };

    Ok(resume_frames)
}
