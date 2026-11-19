# bboxy450-term-mobile

An iOS Simulator and Android emulator app for answering seat questions. The
current Inbox uses an in-process `Questions` fixture and keeps answers for the
life of the app process. It draws the same Rust UI kit as
[bboxy450-term](https://github.com/bboxy450/bboxy450-term), pinned under
`vendor/bboxy450-term`.

Start on an Apple silicon Mac with Xcode and an iOS simulator runtime from iOS 17 through 26,
Homebrew, Rustup, Python 3.11 or newer, and cargo-nextest. Initialize a fresh checkout
with `git submodule update --init`.

## `make help`

Lists the build, verification and pin commands below.

## `make test`

Runs nextest and renders the portrait Inbox through the UI kit's headless GPU
device to `target/smoke/host.png`. The test measures the fixture question in the
draw list and compares the rendered frame to a frame with that question removed.

## `make test-formal`

Exports a bounded TLA+ model of one Inbox row and its foreground surface
through Yarn. It compares all reachable states and action-labeled transitions
against pinned Java TLC. The model assumes a redraw eventually runs while the
app stays foregrounded. The broken control retains the surface on suspend and
must violate the surface invariant. The coverage inventory lists behavior checked
through the implementation. Results and source hashes are in
`target/formal/evidence.json`. This gate also runs under `make test`.

## `make test-licenses`

Checks source notice headers and the shared license and bundled-font notice.
This gate also runs under `make test`.

## `make ios-sim`

Builds the Rust static library for `aarch64-apple-ios-sim` and links it with
Clang into `apps/ios/bboxy450-term-mobile.app`. The bundle uses an Info.plist
and a C entry point. It needs no Xcode project or Apple account. The smoke test
uses the newest installed runtime before iOS 27, whose scene lifecycle is
required by UIKit and is unavailable in winit 0.30.

## `make ios-smoke`

Creates or boots the dedicated `bboxy450-smoke` simulator, installs the app,
waits for its first-frame marker, and checks the captured Inbox pixels in
`target/smoke/ios.png`. The launch log is `target/smoke/ios.log`.

## `make android-tools`

Installs Homebrew's OpenJDK 17, Android command-line tools, platform-tools,
API 35, build tools, the NDK, emulator and an ARM64 Google APIs system image.
`ANDROID_HOME` defaults to `~/Android/Sdk`. The command checks disk space
before installing, accepts SDK licenses through stdin, installs cargo-ndk,
and prints the sizes of the SDK, image, emulator, NDK and JDK.

## `make android-apk`

Builds a Rust ARM64 `.so` with cargo-ndk and packages it as a NativeActivity
debug APK at `target/android/bboxy450-term-mobile.apk`. The build creates a
local throwaway APK signing key under `target/android`.

## `make android-smoke`

Boots the dedicated headless `bboxy450-smoke` AVD under `target/android/avd`,
installs the APK, waits for the first frame, and checks
`target/smoke/android.png`. It taps the first answer button, backgrounds
and resumes the app, and confirms the answer survives surface recreation in
`target/smoke/android-answered.png`. It terminates the emulator it started.

## `make bump-term [REV=sha]`

Moves the shared UI kit pin to its `origin/main`, or a specified commit on
that branch, refreshes the Cargo resolution, and stages the gitlink and lockfile.
