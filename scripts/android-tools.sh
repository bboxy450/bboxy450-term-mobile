# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term-mobile — https://github.com/bboxy450/bboxy450-term-mobile
# Credit copied, adapted and translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term-mobile/blob/main/ATTRIBUTION.md

set -eu

# macOS df reports available blocks in KiB, with the same free space as df -h.
df -h /
available=$(df -k / | awk 'NR == 2 {print $4}')
if [ "$available" -lt 20971520 ]; then
    echo "Android tools require at least 20 GiB available on /" >&2
    exit 1
fi

export HOMEBREW_NO_AUTO_UPDATE=1
if ! brew list --versions openjdk@17 >/dev/null; then
    brew install openjdk@17
fi
export JAVA_HOME="$(brew --prefix openjdk@17)/libexec/openjdk.jdk/Contents/Home"
export PATH="$JAVA_HOME/bin:$PATH"

tools="$ANDROID_HOME/cmdline-tools/latest"
if [ ! -x "$tools/bin/sdkmanager" ]; then
    mkdir -p "$ANDROID_HOME"
    tmp=$(mktemp -d "$ANDROID_HOME/.cmdline-tools.XXXXXX")
    trap 'rm -rf "$tmp"' EXIT
    curl --fail --location --retry 2 \
        https://dl.google.com/android/repository/commandlinetools-mac-15641748_latest.zip \
        --output "$tmp/tools.zip"
    echo "b62a5d8cf63ded173b47be867be4ee058ceda6df  $tmp/tools.zip" | shasum -a 1 -c -
    unzip -q "$tmp/tools.zip" -d "$tmp"
    mkdir -p "$ANDROID_HOME/cmdline-tools"
    mv "$tmp/cmdline-tools" "$tools"
    rm -rf "$tmp"
    trap - EXIT
fi

sdkmanager="$tools/bin/sdkmanager"
yes | "$sdkmanager" --sdk_root="$ANDROID_HOME" --licenses >/dev/null
"$sdkmanager" --sdk_root="$ANDROID_HOME" --install platform-tools emulator \
    "platforms;android-$ANDROID_API" "build-tools;$ANDROID_BUILD_TOOLS" \
    "ndk;$ANDROID_NDK_VERSION" "$ANDROID_IMAGE"
if ! cargo ndk --version >/dev/null 2>&1; then
    cargo install --locked -j 2 cargo-ndk
fi

du -sh "$ANDROID_HOME" "$ANDROID_HOME/emulator" "$ANDROID_HOME/ndk/$ANDROID_NDK_VERSION" \
    "$ANDROID_HOME/system-images/android-$ANDROID_API" "$(brew --cellar openjdk@17)"
