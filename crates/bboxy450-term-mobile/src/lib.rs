// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term-mobile — https://github.com/bboxy450/bboxy450-term-mobile
// Credit copied, adapted and translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term-mobile/blob/main/ATTRIBUTION.md

#[cfg(any(test, target_os = "ios", target_os = "android"))]
mod inbox;
#[cfg(any(target_os = "ios", target_os = "android"))]
mod mobile;
