# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term-mobile — https://github.com/bboxy450/bboxy450-term-mobile
# Credit copied, adapted and translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term-mobile/blob/main/ATTRIBUTION.md

.DEFAULT_GOAL := help
.NOTPARALLEL:

export ANDROID_HOME ?= $(HOME)/Android/Sdk
export ANDROID_API := 35
export ANDROID_BUILD_TOOLS := 35.0.0
export ANDROID_NDK_VERSION := 28.2.13676358
export ANDROID_IMAGE := system-images;android-$(ANDROID_API);google_apis;arm64-v8a
export CARGO_TARGET_DIR := $(CURDIR)/target
export CARGO_BUILD_JOBS ?= 2
export REV

.PHONY: help test test-formal test-licenses ios-sim ios-smoke android-tools android-apk android-smoke bump-term
help:
	@printf '%s\n' \
	  'make test                      Run fixture tests and render the Inbox on the host' \
	  'make test-formal               Check the bounded fixture lifecycle model' \
	  'make test-licenses             Verify license and attribution notices' \
	  'make ios-sim                   Build the unsigned iOS Simulator bundle' \
	  'make ios-smoke                 Boot, launch and capture the iOS Simulator' \
	  'make android-tools             Provision the Android SDK, NDK, emulator and JDK' \
	  'make android-apk               Build the NativeActivity debug APK' \
	  'make android-smoke             Boot, launch and capture the headless Android emulator' \
	  'make bump-term [REV=sha]        Move and stage the shared kit pin (default origin/main)'

android-tools:
	sh scripts/android-tools.sh

IOS_TARGET := aarch64-apple-ios-sim
IOS_BUNDLE := apps/ios/bboxy450-term-mobile.app
ANDROID_OUT := target/android
ANDROID_BUILD := $(ANDROID_HOME)/build-tools/$(ANDROID_BUILD_TOOLS)
android-apk android-smoke: export JAVA_HOME = $(shell brew --prefix openjdk@17)/libexec/openjdk.jdk/Contents/Home

test: test-licenses test-formal
	cargo nextest run --locked --success-output immediate --package bboxy450-term-mobile

ios-sim:
	SDKROOT="$$(xcrun --sdk iphonesimulator --show-sdk-path)" \
	  cargo build --locked --target $(IOS_TARGET)
	mkdir -p "$(IOS_BUNDLE)"
	cp apps/ios/Info.plist LICENSE ATTRIBUTION.md "$(IOS_BUNDLE)/"
	xcrun --sdk iphonesimulator clang -Wl,-S -target arm64-apple-ios17.0-simulator \
	  -isysroot "$$(xcrun --sdk iphonesimulator --show-sdk-path)" apps/ios/main.c \
	  target/$(IOS_TARGET)/debug/libbboxy450_term_mobile.a \
	  -framework UIKit -framework Foundation -framework Metal -framework QuartzCore \
	  -framework CoreGraphics -framework CoreText -framework CoreFoundation \
	  -framework IOSurface -framework Security \
	  -o "$(IOS_BUNDLE)/bboxy450-term-mobile"

ios-smoke: ios-sim
	python3 scripts/smoke.py ios

android-apk:
	ANDROID_NDK_HOME="$(ANDROID_HOME)/ndk/$(ANDROID_NDK_VERSION)" \
	  cargo ndk --target arm64-v8a --platform 28 --output-dir "$(ANDROID_OUT)/native/lib" build --locked
	mkdir -p "$(ANDROID_OUT)"
	"$(ANDROID_BUILD)/aapt2" link --manifest apps/android/AndroidManifest.xml \
	  -I "$(ANDROID_HOME)/platforms/android-$(ANDROID_API)/android.jar" -o "$(ANDROID_OUT)/unpacked.apk"
	cd "$(ANDROID_OUT)/native" && zip -q -u ../unpacked.apk lib/arm64-v8a/libbboxy450_term_mobile.so
	zip -q -j "$(ANDROID_OUT)/unpacked.apk" LICENSE ATTRIBUTION.md \
	  vendor/bboxy450-term/crates/bbt-render/tests/fonts/OFL.txt
	"$(ANDROID_BUILD)/zipalign" -f 4 "$(ANDROID_OUT)/unpacked.apk" "$(ANDROID_OUT)/aligned.apk"
	test -f "$(ANDROID_OUT)/smoke.p12" || "$(JAVA_HOME)/bin/keytool" -genkeypair \
	  -storetype PKCS12 -keystore "$(ANDROID_OUT)/smoke.p12" -storepass bboxy450-smoke \
	  -keypass bboxy450-smoke -alias smoke -dname 'CN=bboxy450 mobile smoke' -keyalg RSA -validity 3650
	"$(ANDROID_BUILD)/apksigner" sign --ks "$(ANDROID_OUT)/smoke.p12" --ks-pass pass:bboxy450-smoke \
	  --out "$(ANDROID_OUT)/bboxy450-term-mobile.apk" "$(ANDROID_OUT)/aligned.apk"

android-smoke: android-apk
	python3 scripts/smoke.py android

# Inspired by bboxy450-term's bump-harness (Makefile, scripts/bump-harness.py).
# https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
bump-term:
	python3 scripts/bump-term.py "$$REV"

test-formal:
	rm -f target/formal/evidence.json
	cargo run --locked --example verify -- target/formal/models
	python3 scripts/formal.py

test-licenses:
	cmp LICENSE vendor/bboxy450-term/LICENSE
	test -s vendor/bboxy450-term/crates/bbt-render/tests/fonts/OFL.txt
	@for source in Makefile Cargo.toml rust-toolchain.toml crates/*/Cargo.toml \
	  crates/*/src/*.rs crates/*/examples/*.rs apps/ios/main.c apps/ios/Info.plist \
	  apps/android/AndroidManifest.xml scripts/*.py scripts/*.sh verification/coverage.toml; do \
	    grep -Fq 'https://github.com/bboxy450/bboxy450-term-mobile/blob/main/ATTRIBUTION.md' "$$source" \
	      || { echo "Missing attribution notice: $$source" >&2; exit 1; }; \
	done
