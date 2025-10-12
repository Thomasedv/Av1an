use anyhow::{Context, Error};
use av1_grain::TransferFunction;
use av_decoders::VapoursynthDecoder;
use colored::*;
use itertools::Itertools;
use nom::error::context;
use num_traits::cast::ToPrimitive;
use rand::{prelude::SliceRandom, rng};
use std::env::args;
use std::process::{Child, Output};
use std::{
    borrow::Cow,
    cmp::{self, Reverse},
    ffi::OsString,
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    iter,
    path::{Path, PathBuf},
    process::{exit, ChildStderr, Command, Stdio},
    sync::{
        atomic::{self, AtomicBool, AtomicUsize},
        mpsc, Arc, Mutex,
    },
    thread::{self, available_parallelism},
};
use tracing::{debug, error, info, warn};

use crate::bootstrap::prepare_environment;
use crate::ffmpeg::FFPixelFormat;
use crate::vapoursynth::VapoursynthPlugins;
use crate::{
    broker::{Broker, EncoderCrash},
    chunk::Chunk,
    concat::{self, ConcatMethod},
    create_dir, determine_workers,
    ffmpeg::{compose_ffmpeg_pipe, get_num_frames},
    get_done, init_done, into_vec,
    metrics::vmaf,
    progress_bar::{
        finish_progress_bar, inc_bar, inc_mp_bar, init_multi_progress_bar, init_progress_bar,
        reset_bar_at, reset_mp_bar_at, set_audio_size, update_mp_chunk, update_mp_msg_encoder,
        update_progress_bar_estimates,
    },
    read_chunk_queue, save_chunk_queue,
    scenes::{Scene, SceneFactory, ZoneOptions},
    settings::{EncodeArgs, InputPixelFormat},
    split::segment,
    vapoursynth::create_vs_file,
    zones::parse_zones,
    ChunkMethod, ChunkOrdering, DashMap, DoneJson, Encoder, Input, PixelFormat, ScenecutMethod,
    SplitMethod, TargetQuality, Verbosity,
};

#[derive(Debug)]
pub struct Av1anEncode {
    pub frames: usize,
    pub vapoursynth_options: VapoursynthOption,
    pub av1an_config: Av1anConfiguration,
    pub target_quality_options: TargetQualityOptions,
    pub resume_options: ResumeOption,
    pub encoder_options: EncoderOption,
    pub(crate) scene_factory: SceneFactory,
}

#[derive(Debug)]
pub struct Av1anConfiguration {
    pub input: Input,
    pub proxy_input: Option<Input>,
    pub output_file: String,
    pub chunk_method: ChunkMethod,
    pub chunk_order: ChunkOrdering,
    pub scaler: String,
    pub scenes: Option<PathBuf>,
    pub split_method: SplitMethod,
    pub sc_pix_format: Option<FFPixelFormat>,
    pub sc_method: ScenecutMethod,
    pub sc_only: bool,
    pub sc_downscale_height: Option<usize>,
    pub extra_splits_len: Option<usize>,
    pub min_scene_len: usize,
    pub force_keyframes: Vec<usize>,
    pub ignore_frame_mismatch: bool,
    pub workers: usize,
    pub set_thread_affinity: Option<usize>,
    pub concat: ConcatMethod,
}

impl Av1anConfiguration {
    fn from_args(args: &EncodeArgs) -> Self {
        Av1anConfiguration {
            input: args.input.clone(),
            proxy_input: args.proxy.clone(),
            chunk_method: args.chunk_method,
            chunk_order: args.chunk_order,
            concat: args.concat,
            scaler: args.scaler.clone(),
            scenes: args.scenes.clone(),
            split_method: args.split_method.clone(),
            sc_pix_format: args.sc_pix_format.clone(),
            sc_method: args.sc_method,
            sc_only: args.sc_only,
            sc_downscale_height: args.sc_downscale_height,
            extra_splits_len: args.extra_splits_len,
            min_scene_len: args.min_scene_len,
            force_keyframes: args.force_keyframes.clone(),
            ignore_frame_mismatch: args.ignore_frame_mismatch,
            workers: args.workers,
            set_thread_affinity: args.set_thread_affinity,
            output_file: args.output_file.clone(),
        }
    }
}

#[derive(Debug)]
pub struct EncoderOption {
    pub passes: u8,
    pub video_params: Vec<String>,
    pub verbosity: Verbosity,
    pub no_defaults: bool,
    pub tile_auto: bool,
    pub tiles: (u32, u32), /* tile (cols, rows) count; log2 will be applied later
                            * for specific encoders */
    pub encoder: Encoder,

    // FFmpeg params
    pub ffmpeg_filter_args: Vec<String>,
    pub audio_params: Vec<String>,
    pub input_pix_format: InputPixelFormat,
    pub output_pix_format: PixelFormat,
}

impl EncoderOption {
    fn from_args(args: &EncodeArgs) -> Self {
        Self {
            passes: args.passes,
            video_params: args.video_params.clone(),
            verbosity: args.verbosity,
            no_defaults: args.no_defaults,
            tile_auto: args.tile_auto,
            tiles: args.tiles,
            encoder: args.encoder,
            ffmpeg_filter_args: args.ffmpeg_filter_args.clone(),
            audio_params: args.audio_params.clone(),
            input_pix_format: args.input_pix_format,
            output_pix_format: args.output_pix_format,
        }
    }
}

#[derive(Debug)]
pub struct VapoursynthOption {
    pub vs_script: Option<PathBuf>,
    pub vs_proxy_script: Option<PathBuf>,
    pub vapoursynth_plugins: Option<VapoursynthPlugins>,
}

#[derive(Debug)]
pub struct TargetQualityOptions {
    pub target_quality: TargetQuality,
    pub vmaf: bool,
    pub vmaf_path: Option<PathBuf>,
    pub vmaf_res: String,
    pub probe_res: Option<String>,
    pub vmaf_threads: Option<usize>,
    pub vmaf_filter: Option<String>,
}

impl TargetQualityOptions {
    fn from_args(args: &EncodeArgs) -> Self {
        Self {
            target_quality: args.target_quality.clone(),
            vmaf: args.vmaf,
            vmaf_path: args.vmaf_path.clone(),
            vmaf_res: args.vmaf_res.clone(),
            probe_res: args.probe_res.clone(),
            vmaf_threads: args.vmaf_threads.clone(),
            vmaf_filter: args.vmaf_filter.clone(),
        }
    }
}

#[derive(Debug)]
pub struct ResumeOption {
    pub temp: String,
    pub resume: bool,
    pub keep: bool,
    pub force: bool,
}

impl ResumeOption {
    pub fn fromArgs(encode_args: &EncodeArgs) -> Self {
        Self {
            temp: encode_args.temp.clone(),
            resume: encode_args.resume,
            force: encode_args.force,
            keep: encode_args.keep,
        }
    }
}

impl Av1anEncode {
    #[tracing::instrument(level = "debug")]
    pub fn new(mut args: EncodeArgs) -> anyhow::Result<Self> {
        args.validate()?;

        let av1an_config = Av1anConfiguration::from_args(&args);

        let target_quality_options = TargetQualityOptions::from_args(&args);
        let encoder_options = EncoderOption::from_args(&args);
        let mut resume_options = ResumeOption::fromArgs(&args);
        let mut frames = prepare_environment(&mut resume_options)?;

        if frames == 0 {
            frames = av1an_config.input.get_clip_info()?.num_frames;
        }

        let vapoursynth_options = VapoursynthOption {
            vs_script: None,
            vs_proxy_script: None,
            vapoursynth_plugins: args.vapoursynth_plugins,
        };

        let this = Self {
            frames,
            vapoursynth_options,
            av1an_config,
            scene_factory: SceneFactory::new(),
            target_quality_options,
            resume_options,
            encoder_options,
        };
        Ok(this)
    }

    fn create_vs_script(
        vs_input: &Input,
        resume_option: &ResumeOption,
        encoding_configuration: &Av1anConfiguration,
    ) -> Result<PathBuf, Error> {
        let script_path = match vs_input {
            Input::VapourSynth { path, .. } => path.clone(),
            Input::Video { path, is_proxy, .. } => {
                let (script_path, _) = create_vs_file(
                    &resume_option.temp,
                    path,
                    encoding_configuration.chunk_method,
                    encoding_configuration.sc_downscale_height,
                    encoding_configuration.sc_pix_format,
                    &encoding_configuration.scaler,
                    *is_proxy,
                )?;
                script_path
            },
        };

        let variables_map = vs_input.as_vspipe_args_hashmap()?;
        let decoder = match vs_input {
            Input::VapourSynth { path, .. } => {
                let mut dec = VapoursynthDecoder::from_file(path)?;
                dec.set_variables(variables_map)?;
                av_scenechange::Decoder::from_decoder_impl(av_decoders::DecoderImpl::Vapoursynth(
                    dec,
                ))?
            },
            video_input => av_scenechange::Decoder::from_script(
                &video_input.as_script_text(
                    encoding_configuration.sc_downscale_height,
                    encoding_configuration.sc_pix_format,
                    Some(&encoding_configuration.scaler),
                )?,
                Some(variables_map),
            )?,
        };
        // Getting the details will evaluate the script and produce the VapourSynth
        // cache file
        decoder.get_video_details();

        Ok::<PathBuf, Error>(script_path)
    }

    #[tracing::instrument(skip(self))]
    pub fn encode_file(&mut self) -> anyhow::Result<()> {
        let initial_frames =
            get_done().done.iter().map(|ref_multi| ref_multi.frames).sum::<usize>();

        // Technically we should check if the vapoursynth cache file exists rather than
        // !self.resume, but the code still works if we are resuming and the
        // cache file doesn't exist (as it gets generated when vspipe is first
        // called), so it's not worth adding all the extra complexity.
        if (self.av1an_config.input.is_vapoursynth()
            || (self.av1an_config.input.is_video()
                && matches!(
                    self.av1an_config.chunk_method,
                    ChunkMethod::LSMASH
                        | ChunkMethod::FFMS2
                        | ChunkMethod::DGDECNV
                        | ChunkMethod::BESTSOURCE
                )))
            && !self.resume_options.resume
        {
            self.vapoursynth_options.vs_script = Some(Self::create_vs_script(
                &self.av1an_config.input,
                &self.resume_options,
                &self.av1an_config,
            )?);
        }
        if let Some(proxy) = &self.av1an_config.proxy_input {
            if proxy.is_vapoursynth()
                || (proxy.is_video()
                    && matches!(
                        self.av1an_config.chunk_method,
                        ChunkMethod::LSMASH
                            | ChunkMethod::FFMS2
                            | ChunkMethod::DGDECNV
                            | ChunkMethod::BESTSOURCE
                    )
                    && !self.resume_options.resume)
            {
                self.vapoursynth_options.vs_proxy_script = Some(Self::create_vs_script(
                    proxy,
                    &self.resume_options,
                    &self.av1an_config,
                )?);
            }
        }

        let clip_info = self.av1an_config.input.get_clip_info()?;
        let res = clip_info.resolution;
        let fps_ratio = clip_info.frame_rate;
        let fps = fps_ratio.to_f64().expect("fps_ratio is not NaN");
        let format = clip_info.format_info;
        let tfc = clip_info.transfer_function_params_adjusted(&self.encoder_options.video_params);
        info!(
            "Input: {}x{} @ {:.3} fps, {}, {}",
            res.0,
            res.1,
            fps,
            match format {
                InputPixelFormat::VapourSynth { bit_depth } => format!("{bit_depth} BPC"),
                InputPixelFormat::FFmpeg { format } => format!("{format:?}"),
            },
            match tfc {
                TransferFunction::SMPTE2084 => "HDR",
                TransferFunction::BT1886 => "SDR",
            }
        );

        let splits = self.split_routine()?.to_vec();

        if self.av1an_config.sc_only {
            debug!("scene detection only");

            if let Err(e) = fs::remove_dir_all(&self.resume_options.temp) {
                warn!("Failed to delete temp directory: {e}");
            }

            exit(0);
        }

        let (chunk_queue, total_chunks) = self.load_or_gen_chunk_queue(&splits)?;

        let mut chunks_done = 0;
        if self.resume_options.resume {
            chunks_done = get_done().done.len();
            info!(
                "encoding resumed with {}/{} chunks completed ({} remaining)",
                chunks_done,
                total_chunks,
                chunk_queue.len()
            );
        }

        crossbeam_utils::thread::scope(|s| -> anyhow::Result<()> {
            // vapoursynth audio is currently unsupported
            let audio_thread = (self.av1an_config.input.is_video()
                && (!self.resume_options.resume
                    || !get_done().audio_done.load(atomic::Ordering::SeqCst)))
            .then(|| {
                let input = self.av1an_config.input.as_video_path();
                let temp = self.resume_options.temp.as_str();
                let audio_params = self.encoder_options.audio_params.as_slice();
                s.spawn(move |_| -> anyhow::Result<_> {
                    let audio_output = crate::ffmpeg::encode_audio(input, temp, audio_params)?;
                    get_done().audio_done.store(true, atomic::Ordering::SeqCst);

                    let progress_file = Path::new(temp).join("done.json");
                    let mut progress_file = File::create(progress_file)?;
                    progress_file.write_all(serde_json::to_string(get_done())?.as_bytes())?;

                    if let Some(ref audio_output) = audio_output {
                        let audio_size = audio_output.metadata()?.len();
                        set_audio_size(audio_size);
                    }

                    Ok(audio_output.is_some())
                })
            });

            if self.av1an_config.workers == 0 {
                self.av1an_config.workers = determine_workers(
                    &self.av1an_config.input,
                    self.encoder_options.tiles,
                    self.av1an_config.chunk_method,
                    self.encoder_options.encoder,
                    self.encoder_options.output_pix_format,
                )? as usize;
            }
            self.av1an_config.workers = cmp::min(self.av1an_config.workers, chunk_queue.len());

            info!(
                "\n{}{} {} {}{} {} {}{} {} {}{} {}\n{}: {}",
                "Q".green().bold(),
                "ueue".green(),
                format!("{len}", len = chunk_queue.len()).green().bold(),
                "W".blue().bold(),
                "orkers".blue(),
                format!("{workers}", workers = self.av1an_config.workers).blue().bold(),
                "E".purple().bold(),
                "ncoder".purple(),
                format!("{encoder}", encoder = self.encoder_options.encoder).purple().bold(),
                "P".purple().bold(),
                "asses".purple(),
                format!("{passes}", passes = self.encoder_options.passes).purple().bold(),
                "Params".bold(),
                self.encoder_options.video_params.join(" ").dimmed()
            );

            if self.encoder_options.verbosity == Verbosity::Normal {
                init_progress_bar(
                    self.frames as u64,
                    initial_frames as u64,
                    Some((chunks_done as u32, total_chunks as u32)),
                );
                reset_bar_at(initial_frames as u64);
            } else if self.encoder_options.verbosity == Verbosity::Verbose {
                init_multi_progress_bar(
                    self.frames as u64,
                    self.av1an_config.workers,
                    initial_frames as u64,
                    (chunks_done as u32, total_chunks as u32),
                );
                reset_mp_bar_at(initial_frames as u64);
            }

            if chunks_done > 0 {
                update_progress_bar_estimates(
                    fps,
                    self.frames,
                    self.encoder_options.verbosity,
                    (chunks_done as u32, total_chunks as u32),
                );
            }

            let broker = Broker {
                chunk_queue,
                project: self,
            };

            let (tx, rx) = mpsc::channel();
            let handle = s.spawn(|_| -> anyhow::Result<()> {
                broker.encoding_loop(
                    tx,
                    self.av1an_config.set_thread_affinity,
                    total_chunks as u32,
                )?;
                Ok(())
            });

            // Queue::encoding_loop only sends a message if there was an error (meaning a
            // chunk crashed) more than MAX_TRIES. So, we have to explicitly
            // exit the program if that happens.
            if rx.recv().is_ok() {
                exit(1);
            }

            handle.join().expect("thread should join successfully")?;

            finish_progress_bar();

            // TODO add explicit parameter to concatenation functions to control whether
            // audio is also muxed in
            let _audio_output_exists = if let Some(audio_thread) = audio_thread {
                audio_thread.join().expect("thread should join successfully")?
            } else {
                false
            };

            debug!(
                "encoding finished, concatenating with {concat}",
                concat = self.av1an_config.concat
            );

            match self.av1an_config.concat {
                ConcatMethod::Ivf => {
                    concat::ivf(
                        &Path::new(&self.resume_options.temp).join("encode"),
                        self.av1an_config.output_file.as_ref(),
                    )?;
                },
                ConcatMethod::MKVMerge => {
                    concat::mkvmerge(
                        self.resume_options.temp.as_ref(),
                        self.av1an_config.output_file.as_ref(),
                        self.encoder_options.encoder,
                        total_chunks,
                        if self.av1an_config.ignore_frame_mismatch {
                            info!(
                                "`--ignore-frame-mismatch` set. Don't force output FPS, as an FPS \
                                 changing filter might have been applied."
                            );
                            None
                        } else {
                            debug!(
                                "`--ignore-frame-mismatch` not set. Forcing output FPS to \
                                 {fps_ratio} with mkvmerge."
                            );
                            Some(fps_ratio)
                        },
                    )?;
                },
                ConcatMethod::FFmpeg => {
                    concat::ffmpeg(
                        self.resume_options.temp.as_ref(),
                        self.av1an_config.output_file.as_ref(),
                    )?;
                },
            }

            if self.target_quality_options.vmaf {
                let vmaf_res = if self.target_quality_options.target_quality.vmaf_res == "inputres"
                {
                    let inputres = self.av1an_config.input.get_clip_info()?.resolution;
                    format!("{width}x{height}", width = inputres.0, height = inputres.1)
                } else {
                    self.target_quality_options.target_quality.vmaf_res.clone()
                };

                let vmaf_model = self.target_quality_options.vmaf_path.as_deref().or(self
                    .target_quality_options
                    .target_quality
                    .model
                    .as_deref());
                let vmaf_scaler = "bicubic";
                let vmaf_filter = self.target_quality_options.vmaf_filter.as_deref().or(self
                    .target_quality_options
                    .target_quality
                    .vmaf_filter
                    .as_deref());

                if self.target_quality_options.vmaf {
                    let vmaf_threads = available_parallelism().map_or(1, std::num::NonZero::get);

                    if let Err(e) = vmaf::plot(
                        self.av1an_config.output_file.as_ref(),
                        &self.av1an_config.input,
                        vmaf_model,
                        &vmaf_res,
                        vmaf_scaler,
                        1,
                        vmaf_filter,
                        vmaf_threads,
                        &self.target_quality_options.target_quality.probing_vmaf_features,
                    ) {
                        error!("VMAF calculation failed with error: {e}");
                    }
                }
            }

            if !Path::new(&self.av1an_config.output_file).exists() {
                warn!(
                    "Concatenation failed for unknown reasons! Temp folder will not be deleted: \
                     {temp}",
                    temp = &self.resume_options.temp.
                );
            } else if !self.resume_options.keep {
                if let Err(e) = fs::remove_dir_all(&self.resume_options.temp) {
                    warn!("Failed to delete temp directory: {e}");
                }
            }

            Ok(())
        })
        .expect("thread should spawn successfully")?;

        Ok(())
    }

    #[tracing::instrument(level = "debug")]
    fn read_queue_files(source_path: &Path) -> anyhow::Result<Vec<PathBuf>> {
        let mut queue_files = fs::read_dir(source_path)
            .with_context(|| {
                format!(
                    "Failed to read queue files from source path {}",
                    source_path.display()
                )
            })?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, _>>()?;

        queue_files.retain(|file| {
            file.is_file() && matches!(file.extension().map(|ext| ext == "mkv"), Some(true))
        });
        concat::sort_files_by_filename(&mut queue_files);

        Ok(queue_files)
    }

    fn build_encoder_command(
        chunk: &Chunk,
        current_pass: u8,
        fpf_file: &Path,
    ) -> anyhow::Result<Vec<String>> {
        let mut cmd = if chunk.passes == 1 {
            chunk.encoder.compose_1_1_pass(chunk.video_params.clone(), chunk.output())
        } else if current_pass == 1 {
            chunk.encoder.compose_1_2_pass(
                chunk.video_params.clone(),
                fpf_file.to_string_lossy().as_ref(),
            )
        } else {
            chunk.encoder.compose_2_2_pass(
                chunk.video_params.clone(),
                fpf_file.to_string_lossy().as_ref(),
                chunk.output(),
            )
        };

        if let Some(tq_cq) = chunk.tq_cq {
            cmd = chunk.encoder.man_command(cmd, tq_cq as usize);
        }
        Ok(cmd)
    }

    fn spawn_source_and_ffmpeg(
        &self,
        chunk: &Chunk,
    ) -> Result<(Stdio, ChildStderr, Option<ChildStderr>), (Error, u64)> {
        // Spawn source process
        let mut source_pipe = if let [source, args @ ..] = &*chunk.source_cmd {
            let mut command = Command::new(source);
            for arg in chunk.input.as_vspipe_args_vec().map_err(|e| (e, 0))? {
                command.args(["-a", &arg]);
            }
            command
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| (e.into(), 0))?
        } else {
            unreachable!()
        };

        let source_pipe_stdout: Stdio =
            source_pipe.stdout.take().expect("source_pipe should have stdout").into();
        let source_pipe_stderr = source_pipe.stderr.take().expect("source_pipe should have stderr");

        // Inner closure to create ffmpeg pipe if needed
        let create_ffmpeg_pipe = |pipe_from: Stdio, source_pipe_stderr: ChildStderr| {
            let ffmpeg_pipe = compose_ffmpeg_pipe(
                self.encoder_options.ffmpeg_filter_args.as_slice(),
                self.encoder_options.output_pix_format.format,
            );

            let mut ffmpeg_pipe = if let [ffmpeg, args @ ..] = &*ffmpeg_pipe {
                Command::new(ffmpeg)
                    .args(args)
                    .stdin(pipe_from)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .map_err(|e| (e.into(), 0))?
            } else {
                unreachable!()
            };

            let ffmpeg_pipe_stdout: Stdio =
                ffmpeg_pipe.stdout.take().expect("ffmpeg_pipe should have stdout").into();
            let ffmpeg_pipe_stderr =
                ffmpeg_pipe.stderr.take().expect("ffmpeg_pipe should have stderr");
            Ok((
                ffmpeg_pipe_stdout,
                source_pipe_stderr,
                Some(ffmpeg_pipe_stderr),
            ))
        };

        // Decide if ffmpeg pipe is needed or not
        if self.encoder_options.ffmpeg_filter_args.is_empty() {
            match &self.encoder_options.input_pix_format {
                InputPixelFormat::FFmpeg { format } => {
                    if self.encoder_options.output_pix_format.format == *format {
                        Ok((source_pipe_stdout, source_pipe_stderr, None))
                    } else {
                        create_ffmpeg_pipe(source_pipe_stdout, source_pipe_stderr)
                    }
                },
                InputPixelFormat::VapourSynth { bit_depth } => {
                    if self.args.output_pix_format.bit_depth == *bit_depth {
                        Ok((source_pipe_stdout, source_pipe_stderr, None))
                    } else {
                        create_ffmpeg_pipe(source_pipe_stdout, source_pipe_stderr)
                    }
                },
            }
        } else {
            create_ffmpeg_pipe(source_pipe_stdout, source_pipe_stderr)
        }
    }

    fn spawn_stderr_reader<'scope, 'env: 'scope, R: std::io::Read + Send + 'static>(
        scope: &'scope thread::Scope<'scope, 'env>,
        reader: BufReader<R>,
        output: Arc<Mutex<String>>,
    ) {
        scope.spawn(move || {
            for line_result in reader.lines() {
                let line = line_result.expect("should read line successfully");
                let mut lock = output.lock().expect("mutex should acquire lock");
                lock.push_str(&line);
                lock.push('\n');
            }
        });
    }

    /// Spawns the encoder process and monitors stderr output.
    fn spawn_encoder_process_and_monitor(
        encoder: &str,
        args: &[String],
        stdin: impl Into<Stdio>,
        chunk: &Chunk,
        current_pass: u8,
        worker_id: usize,
        verbosity: Verbosity,
    ) -> Result<(Output, String, u64), (Error, u64)> {
        let mut enc_pipe = Command::new(encoder)
            .args(args)
            .stdin(stdin)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| (e.into(), 0))?;

        let mut frame = 0;
        let mut reader =
            BufReader::new(enc_pipe.stderr.take().expect("enc_pipe should have stderr"));
        let mut buf = Vec::with_capacity(128);
        let mut enc_stderr = String::with_capacity(128);

        while let Ok(read) = reader.read_until(b'\r', &mut buf) {
            if read == 0 {
                break;
            }

            if let Ok(line) = simdutf8::basic::from_utf8_mut(&mut buf) {
                enc_stderr.push_str(line);
                enc_stderr.push('\n');

                if let Some(new) = chunk.encoder.parse_encoded_frames(line) {
                    if current_pass == chunk.passes {
                        if new > frame {
                            match verbosity {
                                Verbosity::Normal => inc_bar(new - frame),
                                Verbosity::Verbose => inc_mp_bar(new - frame),
                                Verbosity::Quiet => {},
                            }
                            frame = new;
                        }
                    }

                    if verbosity == Verbosity::Verbose {
                        update_mp_msg_encoder(worker_id, current_pass, new, chunk.frames());
                    }
                }
            }

            buf.clear();
        }

        let enc_output = enc_pipe.wait_with_output().expect("enc_pipe should finish");
        Ok((enc_output, enc_stderr, frame))
    }

    fn encode_with_pipes(
        &self,
        chunk: &Chunk,
        current_pass: u8,
        worker_id: usize,
        enc_cmd: &[String],
    ) -> Result<(String, Option<String>, Output, String, u64), (Error, u64)> {
        thread::scope(|scope| {
            let (y4m_pipe, source_pipe_stderr, mut ffmpeg_pipe_stderr) =
                self.spawn_source_and_ffmpeg(chunk)?;

            let source_reader = BufReader::new(source_pipe_stderr);
            let ffmpeg_reader = ffmpeg_pipe_stderr.take().map(BufReader::new);

            let pipe_stderr = Arc::new(Mutex::new(String::with_capacity(128)));
            let p_stdr2 = Arc::clone(&pipe_stderr);

            let ffmpeg_stderr =
                ffmpeg_reader.as_ref().map(|_| Arc::new(Mutex::new(String::with_capacity(128))));

            let f_stdr2 = ffmpeg_stderr.clone();

            Self::spawn_stderr_reader(scope, source_reader, p_stdr2);

            if let Some(ffmpeg_reader) = ffmpeg_reader {
                let f_stdr2 = f_stdr2.expect("f_stdr2 should exist if ffmpeg_reader exists");
                Self::spawn_stderr_reader(scope, ffmpeg_reader, f_stdr2);
            }

            let (enc_output, enc_stderr, frame) = match enc_cmd {
                [encoder, args @ ..] => Self::spawn_encoder_process_and_monitor(
                    encoder,
                    args,
                    y4m_pipe,
                    chunk,
                    current_pass,
                    worker_id,
                    self.args.verbosity,
                )?,
                _ => unreachable!("Encoder command must not be empty"),
            };

            let source_pipe_stderr = pipe_stderr.lock().expect("mutex should acquire lock").clone();
            let ffmpeg_pipe_stderr =
                ffmpeg_stderr.map(|x| x.lock().expect("mutex should acquire lock").clone());

            Ok((
                source_pipe_stderr,
                ffmpeg_pipe_stderr,
                enc_output,
                enc_stderr,
                frame,
            ))
        })
    }

    /// Returns the number of frames encoded if crashed, to reset the progress
    /// bar.
    pub fn encode_chunk(
        &self,
        chunk: &Chunk,
        current_pass: u8,
        worker_id: usize,
        padding: usize,
    ) -> Result<(), (Error, u64)> {
        update_mp_chunk(worker_id, chunk.index, padding);

        let fpf_file = Path::new(&chunk.temp)
            .join("split")
            .join(format!("{name}_fpf", name = chunk.name()));

        let enc_cmd =
            Self::build_encoder_command(chunk, current_pass, &fpf_file).map_err(|e| (e, 0))?;

        let (source_pipe_stderr, ffmpeg_pipe_stderr, enc_output, enc_stderr, frame) =
            Self::encode_with_pipes(self, chunk, current_pass, worker_id, &enc_cmd)?;

        Self::handle_post_encoding_validation(
            chunk,
            current_pass,
            enc_output,
            enc_stderr,
            source_pipe_stderr,
            ffmpeg_pipe_stderr,
            frame,
        )?;

        Ok(())
    }

    pub fn handle_post_encoding_validation(
        chunk: &Chunk,
        current_pass: u8,
        enc_output: Output,
        enc_stderr: String,
        source_pipe_stderr: String,
        ffmpeg_pipe_stderr: Option<String>,
        frame: u64,
    ) -> Result<(), (Error, u64)> {
        if !enc_output.status.success() {
            return Err((
                EncoderCrash {
                    exit_status: enc_output.status,
                    source_pipe_stderr: source_pipe_stderr.into(),
                    ffmpeg_pipe_stderr: ffmpeg_pipe_stderr.map(Into::into),
                    stderr: enc_stderr.into(),
                    stdout: enc_output.stdout.into(),
                }
                .into(),
                frame,
            ));
        }

        if current_pass == chunk.passes {
            let encoded_frames = get_num_frames(chunk.output().as_ref());

            let err_str = match encoded_frames {
                Ok(encoded_frames)
                    if !chunk.ignore_frame_mismatch && encoded_frames != chunk.frames() =>
                {
                    Some(format!(
                        "FRAME MISMATCH: chunk {index}: {encoded_frames}/{expected} \
                     (actual/expected frames)",
                        index = chunk.index,
                        expected = chunk.frames()
                    ))
                },
                Err(error) => Some(format!(
                    "FAILED TO COUNT FRAMES: chunk {index}: {error}",
                    index = chunk.index
                )),
                _ => None,
            };

            if let Some(err_str) = err_str {
                return Err((
                    EncoderCrash {
                        exit_status: enc_output.status,
                        source_pipe_stderr: source_pipe_stderr.into(),
                        ffmpeg_pipe_stderr: ffmpeg_pipe_stderr.map(Into::into),
                        stderr: enc_stderr.into(),
                        stdout: err_str.into(),
                    }
                    .into(),
                    frame,
                ));
            }
        }

        Ok(())
    }

    fn create_encoding_queue(&self, scenes: &[Scene]) -> anyhow::Result<Vec<Chunk>> {
        let mut chunks = match &self.args.input {
            Input::Video { .. } => match self.args.chunk_method {
                ChunkMethod::FFMS2
                | ChunkMethod::LSMASH
                | ChunkMethod::DGDECNV
                | ChunkMethod::BESTSOURCE => {
                    let vs_script =
                        self.vs_script.as_ref().expect("vs_script should exist").as_path();
                    let vs_proxy_script = self.vs_proxy_script.as_deref();
                    self.create_video_queue_vs(scenes, vs_script, vs_proxy_script)?
                },
                ChunkMethod::Hybrid => self.create_video_queue_hybrid(scenes)?,
                ChunkMethod::Select => self.create_video_queue_select(scenes)?,
                ChunkMethod::Segment => self.create_video_queue_segment(scenes)?,
            },
            Input::VapourSynth { path, .. } => {
                self.create_video_queue_vs(scenes, path.as_path(), self.vs_proxy_script.as_deref())?
            },
        };

        match self.args.chunk_order {
            ChunkOrdering::LongestFirst => {
                chunks.sort_unstable_by_key(|chunk| Reverse(chunk.frames()));
            },
            ChunkOrdering::ShortestFirst => {
                chunks.sort_unstable_by_key(Chunk::frames);
            },
            ChunkOrdering::Sequential => {
                // Already in order
            },
            ChunkOrdering::Random => {
                chunks.shuffle(&mut rng());
            },
        }

        Ok(chunks)
    }

    // If we are not resuming, then do scene detection. Otherwise: get scenes from
    // scenes.json and return that.
    fn split_routine(&mut self) -> anyhow::Result<&[Scene]> {
        let scene_file = self.args.scenes.as_ref().map_or_else(
            || Cow::Owned(Path::new(&self.args.temp).join("scenes.json")),
            |path| Cow::Borrowed(path.as_path()),
        );
        if scene_file.exists() && (self.args.scenes.is_some() || self.args.resume) {
            self.scene_factory = SceneFactory::from_scenes_file(&scene_file)?;
        } else {
            let zones = parse_zones(&self.args, self.frames)?;
            self.scene_factory.compute_scenes(&self.args, &zones)?;
            self.scene_factory.write_scenes_to_file(scene_file)?;
        }
        self.frames = self.scene_factory.get_frame_count();
        self.scene_factory.get_split_scenes()
    }

    fn create_select_chunk(
        &self,
        index: usize,
        src_path: &Path,
        start_frame: usize,
        end_frame: usize,
        frame_rate: f64,
        overrides: Option<ZoneOptions>,
    ) -> anyhow::Result<Chunk> {
        assert!(
            start_frame < end_frame,
            "Can't make a chunk with <= 0 frames!"
        );

        let ffmpeg_gen_cmd: Vec<OsString> = into_vec![
            "ffmpeg",
            "-y",
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
            src_path,
            "-vf",
            format!(
                r"select=between(n\,{start}\,{end})",
                start = start_frame,
                end = end_frame - 1
            ),
            "-pix_fmt",
            self.args.output_pix_format.format.to_pix_fmt_string(),
            "-strict",
            "-1",
            "-f",
            "yuv4mpegpipe",
            "-",
        ];

        let output_ext = self.args.encoder.output_extension();

        let mut chunk = Chunk {
            temp: self.args.temp.clone(),
            index,
            input: Input::Video {
                path: src_path.to_path_buf(),
                temp: self.args.temp.clone(),
                chunk_method: ChunkMethod::Select,
                is_proxy: false,
            },
            proxy: self.args.proxy.as_ref().map(|proxy| Input::Video {
                path: proxy.as_path().to_path_buf(),
                temp: self.args.temp.clone(),
                chunk_method: ChunkMethod::Select,
                is_proxy: true,
            }),
            source_cmd: ffmpeg_gen_cmd,
            proxy_cmd: None,
            output_ext: output_ext.to_owned(),
            start_frame,
            end_frame,
            frame_rate,
            video_params: overrides.as_ref().map_or_else(
                || self.args.video_params.clone(),
                |ovr| ovr.video_params.clone(),
            ),
            passes: overrides.as_ref().map_or(self.args.passes, |ovr| ovr.passes),
            encoder: overrides.as_ref().map_or(self.args.encoder, |ovr| ovr.encoder),
            noise_size: self.args.photon_noise_size,
            target_quality: overrides.as_ref().map_or(self.args.target_quality.clone(), |ovr| {
                ovr.target_quality.clone().map_or(self.args.target_quality.clone(), |tq| tq)
            }),
            tq_cq: None,
            ignore_frame_mismatch: self.args.ignore_frame_mismatch,
        };
        chunk.apply_photon_noise_args(
            overrides.map_or(self.args.photon_noise, |ovr| ovr.photon_noise),
            self.args.chroma_noise,
        )?;
        if chunk.target_quality.target.is_some() {
            chunk.tq_cq = Some(chunk.target_quality.per_shot_target_quality(
                &chunk,
                None,
                self.args.vapoursynth_plugins,
            )?);
        }
        Ok(chunk)
    }

    fn create_vs_chunk(
        &self,
        index: usize,
        vs_script: &Path,
        vs_proxy_script: Option<&Path>,
        scene: &Scene,
        frame_rate: f64,
    ) -> anyhow::Result<Chunk> {
        // the frame end boundary is actually a frame that should be included in the
        // next chunk
        let frame_end = scene.end_frame - 1;

        fn gen_vspipe_cmd(vs_script: &Path, scene_start: usize, scene_end: usize) -> Vec<OsString> {
            into_vec![
                "vspipe",
                vs_script,
                "-c",
                "y4m",
                "-",
                "-s",
                scene_start.to_string(),
                "-e",
                scene_end.to_string(),
            ]
        }

        let vspipe_cmd_gen = gen_vspipe_cmd(vs_script, scene.start_frame, frame_end);
        let vspipe_proxy_cmd_gen = vs_proxy_script
            .map(|vs_proxy_script| gen_vspipe_cmd(vs_proxy_script, scene.start_frame, frame_end));

        let output_ext = self.args.encoder.output_extension();

        let mut chunk = Chunk {
            temp: self.args.temp.clone(),
            index,
            input: Input::VapourSynth {
                path: vs_script.to_path_buf(),
                vspipe_args: self.args.input.as_vspipe_args_vec()?,
                script_text: self.args.input.as_script_text(
                    self.args.sc_downscale_height,
                    self.args.sc_pix_format,
                    Some(&self.args.scaler),
                )?,
                is_proxy: false,
            },
            proxy: if let Some(vs_proxy_script) = vs_proxy_script {
                Some(Input::VapourSynth {
                    path: vs_proxy_script.to_path_buf(),
                    vspipe_args: self
                        .args
                        .proxy
                        .as_ref()
                        .expect("proxy should be set")
                        .as_vspipe_args_vec()?,
                    script_text: self
                        .args
                        .proxy
                        .as_ref()
                        .expect("proxy should be set")
                        .as_script_text(
                            self.args.sc_downscale_height,
                            self.args.sc_pix_format,
                            Some(&self.args.scaler),
                        )?,
                    is_proxy: true,
                })
            } else {
                None
            },
            source_cmd: vspipe_cmd_gen,
            proxy_cmd: vspipe_proxy_cmd_gen,
            output_ext: output_ext.to_owned(),
            start_frame: scene.start_frame,
            end_frame: scene.end_frame,
            frame_rate,
            video_params: scene.zone_overrides.as_ref().map_or_else(
                || self.args.video_params.clone(),
                |ovr| ovr.video_params.clone(),
            ),
            passes: scene.zone_overrides.as_ref().map_or(self.args.passes, |ovr| ovr.passes),
            encoder: scene.zone_overrides.as_ref().map_or(self.args.encoder, |ovr| ovr.encoder),
            noise_size: scene.zone_overrides.as_ref().map_or(self.args.photon_noise_size, |ovr| {
                (ovr.photon_noise_width, ovr.photon_noise_height)
            }),
            target_quality: scene.zone_overrides.as_ref().map_or(
                self.args.target_quality.clone(),
                |ovr| {
                    ovr.target_quality.clone().unwrap_or_else(|| self.args.target_quality.clone())
                },
            ),
            tq_cq: None,
            ignore_frame_mismatch: self.args.ignore_frame_mismatch,
        };
        chunk.apply_photon_noise_args(
            scene
                .zone_overrides
                .as_ref()
                .map_or(self.args.photon_noise, |ovr| ovr.photon_noise),
            scene
                .zone_overrides
                .as_ref()
                .map_or(self.args.chroma_noise, |ovr| ovr.chroma_noise),
        )?;
        Ok(chunk)
    }

    fn create_video_queue_vs(
        &self,
        scenes: &[Scene],
        vs_script: &Path,
        vs_proxy_script: Option<&Path>,
    ) -> anyhow::Result<Vec<Chunk>> {
        let frame_rate = self
            .args
            .input
            .clip_info()?
            .frame_rate
            .to_f64()
            .expect("frame rate should not be NaN");
        let chunk_queue: Vec<Chunk> = scenes
            .iter()
            .enumerate()
            .map(|(index, scene)| {
                self.create_vs_chunk(index, vs_script, vs_proxy_script, scene, frame_rate)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(chunk_queue)
    }

    fn create_video_queue_select(&self, scenes: &[Scene]) -> anyhow::Result<Vec<Chunk>> {
        let input = self.args.input.as_video_path();
        let frame_rate = self
            .args
            .input
            .clip_info()?
            .frame_rate
            .to_f64()
            .expect("frame rate should not be NaN");

        let chunk_queue: Vec<Chunk> = scenes
            .iter()
            .enumerate()
            .map(|(index, scene)| {
                self.create_select_chunk(
                    index,
                    input,
                    scene.start_frame,
                    scene.end_frame,
                    frame_rate,
                    scene.zone_overrides.clone(),
                )
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(chunk_queue)
    }

    fn create_video_queue_segment(&self, scenes: &[Scene]) -> anyhow::Result<Vec<Chunk>> {
        let input = self.args.input.as_video_path();
        let frame_rate = self
            .args
            .input
            .clip_info()?
            .frame_rate
            .to_f64()
            .expect("frame rate should not be NaN");

        debug!("Splitting video");
        segment(
            input,
            &self.args.temp,
            &scenes.iter().skip(1).map(|scene| scene.start_frame).collect::<Vec<usize>>(),
        )?;
        debug!("Splitting done");

        let source_path = Path::new(&self.args.temp).join("split");
        let queue_files = Self::read_queue_files(&source_path)?;

        assert!(
            !queue_files.is_empty(),
            "Error: No files found in temp/split, probably splitting not working"
        );

        let chunk_queue: Vec<Chunk> = queue_files
            .iter()
            .enumerate()
            .map(|(index, file)| {
                self.create_chunk_from_segment(
                    index,
                    &file.as_path().to_string_lossy(),
                    frame_rate,
                    scenes[index].zone_overrides.clone(),
                )
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(chunk_queue)
    }

    fn create_video_queue_hybrid(&self, scenes: &[Scene]) -> anyhow::Result<Vec<Chunk>> {
        let input = self.args.input.as_video_path();
        let frame_rate = self
            .args
            .input
            .clip_info()?
            .frame_rate
            .to_f64()
            .expect("frame rate should not be NaN");

        let keyframes = crate::ffmpeg::get_keyframes(input)?;

        let to_split: Vec<usize> = keyframes
            .iter()
            .filter(|kf| scenes.iter().any(|scene| scene.start_frame == **kf))
            .copied()
            .collect();

        debug!("Segmenting video");
        segment(input, &self.args.temp, &to_split[1..])?;
        debug!("Segment done");

        let source_path = Path::new(&self.args.temp).join("split");
        let queue_files = Self::read_queue_files(&source_path)?;

        let kf_list = to_split.iter().copied().chain(iter::once(self.frames)).tuple_windows();

        let mut segments = Vec::with_capacity(scenes.len());
        for (file, (x, y)) in queue_files.iter().zip(kf_list) {
            for s in scenes {
                let s0 = s.start_frame;
                let s1 = s.end_frame;
                if s0 >= x && s1 <= y && s0 < s1 {
                    segments.push((file.as_path(), (s0 - x, s1 - x, s)));
                }
            }
        }

        let chunk_queue: Vec<Chunk> = segments
            .iter()
            .enumerate()
            .map(|(index, &(file, (start, end, scene)))| {
                self.create_select_chunk(
                    index,
                    file,
                    start,
                    end,
                    frame_rate,
                    scene.zone_overrides.clone(),
                )
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(chunk_queue)
    }

    #[tracing::instrument(level = "debug")]
    fn create_chunk_from_segment(
        &self,
        index: usize,
        file: &str,
        frame_rate: f64,
        overrides: Option<ZoneOptions>,
    ) -> anyhow::Result<Chunk> {
        let ffmpeg_gen_cmd: Vec<OsString> = into_vec![
            "ffmpeg",
            "-y",
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
            file.to_owned(),
            "-strict",
            "-1",
            "-pix_fmt",
            self.args.output_pix_format.format.to_pix_fmt_string(),
            "-f",
            "yuv4mpegpipe",
            "-",
        ];

        let output_ext = self.args.encoder.output_extension();

        let num_frames = get_num_frames(Path::new(file))?;

        let mut chunk = Chunk {
            temp: self.args.temp.clone(),
            input: Input::Video {
                path: PathBuf::from(file),
                temp: self.args.temp.clone(),
                chunk_method: ChunkMethod::Segment,
                is_proxy: false,
            },
            proxy: self.args.proxy.as_ref().map(|proxy| Input::Video {
                path: proxy.as_path().to_path_buf(),
                temp: self.args.temp.clone(),
                chunk_method: ChunkMethod::Segment,
                is_proxy: true,
            }),
            source_cmd: ffmpeg_gen_cmd,
            proxy_cmd: None,
            output_ext: output_ext.to_owned(),
            index,
            start_frame: 0,
            end_frame: num_frames,
            frame_rate,
            video_params: overrides.as_ref().map_or_else(
                || self.args.video_params.clone(),
                |ovr| ovr.video_params.clone(),
            ),
            passes: overrides.as_ref().map_or(self.args.passes, |ovr| ovr.passes),
            encoder: overrides.as_ref().map_or(self.args.encoder, |ovr| ovr.encoder),
            noise_size: self.args.photon_noise_size,
            target_quality: overrides.as_ref().map_or(self.args.target_quality.clone(), |ovr| {
                ovr.target_quality.clone().map_or(self.args.target_quality.clone(), |tq| tq)
            }),
            tq_cq: None,
            ignore_frame_mismatch: self.args.ignore_frame_mismatch,
        };
        chunk.apply_photon_noise_args(
            overrides.map_or(self.args.photon_noise, |ovr| ovr.photon_noise),
            self.args.chroma_noise,
        )?;
        Ok(chunk)
    }

    /// Returns unfinished chunks and number of total chunks
    fn load_or_gen_chunk_queue(&self, splits: &[Scene]) -> anyhow::Result<(Vec<Chunk>, usize)> {
        if self.args.resume {
            let mut chunks = read_chunk_queue(self.args.temp.as_ref())?;
            let num_chunks = chunks.len();

            let done = get_done();

            // only keep the chunks that are not done
            chunks.retain(|chunk| !done.done.contains_key(&chunk.name()));

            Ok((chunks, num_chunks))
        } else {
            let chunks = self.create_encoding_queue(splits)?;
            let num_chunks = chunks.len();
            save_chunk_queue(&self.args.temp, &chunks)?;
            Ok((chunks, num_chunks))
        }
    }
}

pub struct EncoderHandle {
    pub child: Child,
    pub stderr_thread: Option<thread::JoinHandle<()>>,
    pub stderr_content: Arc<Mutex<String>>,
}
