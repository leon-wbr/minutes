# Supervised full speaker worker: integration qualification

This is the follow-up to the [full native comparison](full-native-diarization-2026-10-18.md).
PR #1164 adds an explicitly selected, separately built CPU helper. It does not
change the default model, bundle or download the helper, enable named enrollment,
or replace live transcription. Integration remains in qualification; this document
records passing and failing measurements rather than implying a released feature.

## Frozen controls

The three complete, already-seen public AMI development recordings are IS1008a,
TS3004a and IB4001. They are regression controls, not fresh model admission.
The pinned Speakrs source, model files and defaults match the earlier comparison.
Each recording must preserve its input WAV, return four anonymous speakers, keep
DER within one absolute percentage point of its frozen candidate result and finish
with processing time / recording duration at most 0.25. The worker deadline remains
900 seconds. Build time is outside processing time; host decoding, supervised
launch, model loading and processing are included. No speaker-count hint is supplied.

## Portable build receipts

The compiled integration source for these rows is
`f5e643496d720dfb671edb1c68d222df78f26ff9`.
The [native workflow run](https://github.com/silverstein/minutes/actions/runs/39279757314)
binds the worker, host probe, checker, source files and Linux shared OpenBLAS
library by SHA-256. The reports were recovered from completed job logs and their
source hashes checked against that revision. GitHub's aggregate status lagged the
individual job steps while these receipts were inspected.
The numeric [Linux report](full-speaker-worker-2026-10-19/f5e64349-linux.json) and
[ARM64 report](full-speaker-worker-2026-10-19/f5e64349-arm.json) retain those bindings.
The [failed Intel report](full-speaker-worker-2026-10-19/f5e64349-intel-failed.json)
preserves its source bindings and failed runtime gate too.

| Platform | IS1008a seconds / ratio | TS3004a seconds / ratio | IB4001 seconds / ratio | Result |
| --- | ---: | ---: | ---: | --- |
| Linux x86-64 | 212.72 / 0.2254 | 299.48 / 0.2226 | 396.84 / 0.2228 | All frozen gates pass |
| macOS ARM64 | 136.32 / 0.1444 | 188.75 / 0.1403 | 253.08 / 0.1421 | All frozen gates pass |
| macOS Intel | 385.06 / 0.4079 | Not run after failure | Not run after failure | Runtime fails |
| Windows x86-64 | Pending | Pending | Pending | Not qualified |

Passing Linux and ARM64 outputs reproduce DER 9.9478%, 21.1965% and 19.9164%,
respectively, with four speakers in each recording and unchanged WAV hashes.
The Intel first recording reproduces DER 9.9478% and four speakers, and preserves
its WAV; those successes do not compensate for its failed runtime gate. Actual
Unix parent-loss retirement also passes on the two fully passing platforms.

Earlier builds failed runtime on Linux (344.09 seconds / 0.3645) and Intel
(477.23 seconds / 0.5056), and the initial Windows attempt exceeded the host's
900-second deadline. Those failures prompted CPU math-library and PCM transport
changes. They were not erased or converted into passes by relaxing the criteria.

## Next measured optimization

The standalone helper is being evaluated with Rust's
[`-C target-cpu=native`](https://doc.rust-lang.org/rustc/codegen-options/index.html#target-cpu).
It must be built on the machine that will run it, and its compiled objects must
not be reused across CPUs. The Minutes host retains its ordinary portable build.
This changes compiler optimization, not model weights, inference parameters,
speaker counts or acceptance limits. It has not yet established an Intel pass.

The [native-CPU Mac replay](full-speaker-worker-2026-10-19/nativecpu-mac.json)
passes all three controls: 45.48, 64.80 and 86.62 seconds, ratios 0.0482, 0.0482
and 0.0486, with the same DER, four speakers and unchanged inputs. Actual parent
loss retirement passes. Release build, strict helper Clippy and all four helper
tests pass with these compiler flags. The report's source and checker hashes
match the proposed tree; its compiled Rust source is unchanged from `f5e64349`.
This was a separate native Mac from the hosted runner; comparing their times does
not measure the compiler flag's speedup. Linux, Intel and Windows still need their
own native-CPU receipts.

## Scope still unqualified

Anonymous-label protocol checks, cancellation and process-tree retirement are
separate from end-to-end meeting acceptance. Real ASR word alignment and overlap,
multilingual and Dutch conversations, short turns, similar voices, headphone calls,
named false positives, concurrent live-capture latency and signed installed-app
parity remain open. The original benchmark's oracle-word scores are not ASR
accuracy measurements. No universal SOTA, production-default or installed-app
acceptance claim follows from these regression recordings.
