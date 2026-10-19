# Opt-in full speaker refinement worker

This separately built CPU helper runs the pinned Speakrs reconstruction/PLDA/VBx
pipeline. It is outside the main workspace and is not downloaded, auto-selected,
or bundled in Minutes releases. Existing engines and live transcription retain
their current behavior. This is an experimental anonymous-label engine;
production default promotion and named voice enrollment remain unqualified.

Build with the repository's Rust 1.95.0 toolchain:

```bash
cargo build --release --locked --manifest-path crates/speaker-worker/Cargo.toml
```

Code: Apache-2.0 Speakrs at `af8013f95a53a37a554d8abc6e4ae2ab9a614be6`;
Minutes wrapper MIT. Default upstream features are disabled; only `cpu` is
compiled. The helper never downloads models. Its separate Cargo.lock pins the
dependency graph without adding Speakrs to the desktop/CLI dependency graph.

Download and verify the explicit public bundle using the repository evaluation
fetcher, after reviewing model attribution and upstream terms:

```bash
python3 tests/eval/diarization/fetch_full_pipeline.py \
  --manifest tests/eval/diarization/full-native-ami.json --directory /owned/full-native
```

This also fetches public AMI evaluation audio and baseline models. The helper
uses only `/owned/full-native/candidate-models`. The converted model card has
no separate converter license declaration and points to upstream model terms;
this wrapper does not redistribute weights.

Opt in by editing your configuration deliberately (all paths must be absolute):

```toml
[diarization]
engine = "speakrs"
speaker_worker_path = "/absolute/minutes/crates/speaker-worker/target/release/minutes-speaker-worker"
model_path = "/owned/full-native/candidate-models"
```

`minutes setup --diarization` refuses to overwrite this bundle with legacy ONNX
models. Switch `engine` back to `auto` and restore the ordinary ONNX model path
to return to the previous configuration. Neither configuration is changed by
building, fetching, or running the probe below.

```bash
cargo build --release --locked -p minutes-core --no-default-features \
  --features diarize --example full_diarize_file
target/release/examples/full_diarize_file \
  /absolute/minutes-speaker-worker /owned/full-native/candidate-models /owned/meeting.wav
```

The current bridge supports ordinary WAV recordings, up to one hour, resampled
by Minutes to mono 16 kHz. Compressed and anonymous private-capability inputs are
refused rather than weakening their transport rules. Transcription remains
available when the optional speaker worker fails. The worker receives canonical
PCM through stdin and no audio filename. It returns only anonymous activity,
with a digest and sample count binding the response to the input. No embeddings,
names, transcript text, or enrollment writes cross the protocol. Existing
profiles stay in their own namespaces; enrollment for this engine is refused.

The host uses its existing immutable executable binding and process-tree
supervisor: finite deadline, bounded stdout/stderr, explicit cancellation and
Unix process-group / Windows Job retirement. Linux has a 4 GiB virtual-address
ceiling; Windows process/job committed memory is limited to 4 GiB before resume.
macOS installs a 4 GiB virtual-address growth allowance over its dyld baseline
before reading input or constructing models. These memory measures differ;
none is an RSS measurement. A worker-local watchdog also bounds parent loss.
Thread environment settings and Unix nice priority are scheduling hints, not
proof that concurrent capture is unaffected. Model copies are hashed before
inference reopens their private snapshot. Duration, observation, speaker,
segment and output bounds precede acceptance of results.

Preserving overlap in model output does not resolve which overlapping speaker
said an ASR word. Existing conservative attribution remains in place. Real ASR
alignment, multilingual/short-turn/similar-voice/headphone cases, named false
positives, installed-app parity and concurrent live latency remain rollout gates.
