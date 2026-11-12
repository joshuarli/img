# CLI reference

The commands are `optimize`, `convert`, `inspect` and `compare`.
Old codec commands, aliases, backup, suffix, arbitrary transforms and report-file
options are removed. JPEG, PNG, WebP and AVIF are the accepted containers.
Input bytes determine format. Unrecognized explicit inputs fail.

Optimize keeps source format unless `--format` selects another format or `auto`.
The default filename is `<stem>.optimized.<extension>`.
Convert requires a concrete format or an output extension.
Its default filename is `<stem>.converted.<extension>`.
Explicit format and output extension must agree.
Auto requires optimize, conflicts with explicit output and in-place, and uses
`<filename-with-source-extension>.optimized.<winner-extension>` under out-dir.

Multiple inputs and directories require out-dir or in-place for transformations.
Directory children are sorted by native filenames, recursively when requested.
Each input root's relative directory layout is retained under out-dir.
The output subtree is excluded by directory identity, including case variants. Unrelated discovered extensions are skipped.
Symlinks, including traversal entries and destination ancestors, are rejected.
On macOS, destination collision checks honor APFS case/Unicode collation. Linux verification uses case-sensitive ext4.
Duplicate sources, hard-link source aliases, duplicate planned destinations and
preexisting destinations fail before image publication for those jobs.
All possible auto destinations are reserved during planning.

Quote glob arguments for expansion by Rimage. Existing positional paths are
treated literally, including paths containing glob characters.
`--file-list PATH` reads UTF-8 paths, one per line, relative to the list.
A positional `file.list` retains the earlier selection behavior: it supplies
the inputs and ignores other positional inputs. No shell evaluation occurs.
File-list entries are literal, including a filename of `-`.
Empty input sets fail. There is a 100000-file bound and a 128 MiB input/list cap.
Ordinary paths retain native bytes. JSON rejects non-UTF-8 paths before writes.

Default fidelity is measured score 95 and effort balanced. Requested scores
and direct quality must be finite in [0,100]. Quality conflicts with score.
Lossless conflicts with either explicit control and with resizing.
Direct quality selects native encoder controls, not equivalent visual quality:
JPEG uses jpegli-style quality, WebP uses lossy quality with alpha quality 100,
PNG selects 2–256 palette colors, and AVIF maps the interval to native 1–99.
Scored search may evaluate exact PNG and WebP as well as lossy candidates.
Fast, balanced and best permit 8, 24 and 64 total encode attempts respectively.

Keep is the metadata default. Strip is intended to retain required color signaling and
normalizes orientation once. Lossless refuses a strip that must reorder samples.
Known color-preservation defects are recorded in [Color](COLOR.md) and [AVIF](AVIF.md).
Inspect reads headers without decoding planes, including declared unsupported
precision and animation/multiple-image signals, including JPEG MPF and gain-map declarations. JPEG trailing payloads and late application metadata are unsupported for pixel processing. Compare returns a measured score without a threshold.

A single `-` input reads bounded stdin. Binary `-o -` requires one input and a
concrete explicit format. Batch stdout, auto stdout, JSON plus binary stdout,
and in-place stdin are refused. Winners are verified and buffered before stdout.
Broken pipes are output errors. Diagnostics and progress go to stderr.
`--quiet` suppresses success diagnostics; inspect and compare still print their data. `--verbose` prints admission, publication
and evaluated candidate settings. Quiet and verbose conflict.

`--preserve-timestamps` copies source modification time on file outputs.
New files use mode 0600. In-place replacements copy the source's permissions.
New destinations use exclusive destination-directory staging, atomic no-clobber
publication, file sync and parent sync. In-place work requires exclusive writer
ownership of the source until the job finishes. Cooperating Rimage writers lock
the source's parent directory before opening it and hold that lock through
publication. Another process using the same protocol fails with `input.conflict`
when that directory is locked, including when it selects a different file in
the directory. Jobs in one batch wait for each other without holding a second
directory lock. Process exit releases the lock and leaves no lock file.

The identity check before renaming detects many external edits but is not atomic
with the rename. External editors, older Rimage versions, and other writers that
ignore the lock must not modify or replace the source while the job runs; their
changes can otherwise be overwritten.
A post-publication durability failure has kind
`output.committed` and explicitly says the output was committed.

Jobs default to one, capped by requested jobs, CPU parallelism, available memory
and work count. [Still-image limits](CODEC-BOUNDARY.md) describe admission and its
known gaps. Unknown memory uses the retained conservative fallback.
No decoded images are queued. One candidate runs per image.
Codec adapters receive cooperative cancellation where their APIs support it.
The AV1 plane decoder and PNG layout-preserving writer have no cooperative stop API;
shutdown waits for those active calls. INT, TERM, HUP and QUIT request cancellation,
join workers, clean unpublished temporary files, then terminate by the signal.
Stdout writes stop on cancellation even when a pipe remains open and full.
Fatal handlers restore disposition and re-raise without cleanup allocation.
Completed outputs survive cancellation. SIGKILL and power loss have no cleanup
promise, and restart never deletes unknown temporary files.

Exit codes are 0 success, 2 usage, 3 input or score, 4 output, and 5 partial or
mixed errors. Mixed input/output failures return 5 even with zero successes.
Batches are best effort with one transaction per file.

## JSON schema version 1

`--json` emits one stdout object with `schemaVersion`, `results` and `failures`.
Each array follows input order, including with concurrent workers.
Results contain `input`, nullable `output`, `format`, `inputBytes`, nullable
`outputBytes`, nullable `score`, `elapsedMs`, `status` and `warnings`.
Statuses are `written`, `unchanged`, `inspected` and `compared`.
Inspect adds `image` with width, height, bitDepth, alpha, grayscale, ICC/CICP,
orientation, animation, grid, rotationCcwDegrees, mirrorAxis, pixelAspectRatio,
hdrMetadata, EXIF and XMP header details.
Compare adds `reference`. Null denotes an inapplicable or unknown header field.
Source identity has a score only when its representation supports scoring.

Failures contain `input`, nullable `output`, stable `kind`, `message` and
numeric `exitCode`. Consumers may ignore unknown fields. Existing meanings
require a new schema version to change. Elapsed times are diagnostic wall times,
not benchmark measurements.

```json
{"schemaVersion":1,"results":[{"input":"diagram.png","output":"diagram.webp","format":"webp","inputBytes":1200,"outputBytes":900,"score":null,"elapsedMs":4.2,"status":"written","warnings":[]}],"failures":[]}
```

Examples: `rimage optimize diagram.png --lossless --json`,
`rimage convert diagram.png -o diagram.webp --lossless --json`,
`rimage inspect diagram.png --json`, and
`rimage compare diagram.png diagram.webp --json`.
