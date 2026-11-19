// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term-mobile — https://github.com/bboxy450/bboxy450-term-mobile
// Credit copied, adapted and translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term-mobile/blob/main/ATTRIBUTION.md

use bbt_yarn::model::corpus::export;
use std::{fs, path::Path};

macro_rules! inbox_lifecycle {
    ($name:ident, $surface_after_suspend:expr) => {
        bbt_yarn::protocol! {
            pub mod $name {
                domain Reply { Yes, No }
                state {
                    foreground: bool = false;
                    surface: bool = false;
                    ready: bool = false;
                    answer: Option<Reply> = None;
                }
                events { Resume; Suspend; Redraw; Tap(reply: Reply); }
                on Resume {
                    case create when !foreground => {
                        set foreground = true;
                        set surface = true;
                        set ready = false;
                        preserve answer;
                    }
                    case duplicate when foreground => {
                        preserve foreground, surface, ready, answer;
                        ignore AlreadyForeground;
                    }
                }
                on Suspend {
                    case release when true => {
                        set foreground = false;
                        set surface = $surface_after_suspend;
                        set ready = false;
                        preserve answer;
                    }
                }
                on Redraw {
                    #[fair(weak)]
                    case present when foreground && surface && !ready => {
                        set ready = true;
                        preserve foreground, surface, answer;
                    }
                    case skip when !(foreground && surface && !ready) => {
                        preserve foreground, surface, ready, answer;
                        ignore NoPresentationOwed;
                    }
                }
                on Tap(reply) {
                    case record when foreground && surface && answer.is_none() => {
                        set answer = Some(reply);
                        preserve foreground, surface, ready;
                    }
                    case ignore_tap when !(foreground && surface && answer.is_none()) => {
                        preserve foreground, surface, ready, answer;
                        ignore TapUnavailable;
                    }
                }
                #[invariant(id = "INBOX-SURFACE")]
                fn surface_owned_by_foreground() { foreground == surface }
                #[invariant(id = "INBOX-READY")]
                fn ready_has_surface() { !ready || surface }
                #[predicate]
                fn presentation_owed() { foreground && surface && !ready }
                #[predicate]
                fn presentation_resolved() { ready || !foreground }
                #[property(id = "INBOX-PRESENT", leads_to(presentation_owed, presentation_resolved))];
            }
        }
    }
}

inbox_lifecycle!(InboxLifecycle, false);
inbox_lifecycle!(BrokenSurfaceRetention, surface);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .ok_or("usage: verify OUTPUT_DIRECTORY")?;
    let root = Path::new(&output);
    fs::create_dir_all(root)?;
    let mut manifest = String::new();
    export(InboxLifecycle::model(), "pass", root, &mut manifest)?;
    export(
        BrokenSurfaceRetention::model(),
        "safety",
        root,
        &mut manifest,
    )?;
    fs::write(root.join("manifest.tsv"), manifest)?;
    Ok(())
}
