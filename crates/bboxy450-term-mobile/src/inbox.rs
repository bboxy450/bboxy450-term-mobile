// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term-mobile — https://github.com/bboxy450/bboxy450-term-mobile
// Credit copied, adapted and translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term-mobile/blob/main/ATTRIBUTION.md

use bbt_render::chrome::text;
use bbt_ui::{
    Density, Theme,
    atoms::{Atom, AtomCtx, Hairline},
};
use bbt_ui_model::{
    draw::{Color, DrawCtx, DrawList, TextStyle, TypeScale},
    layout::{Axis, Box_, Child, Rect, Size, TextMeasure, TextStyle as MeasureStyle, solve_box},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;

#[derive(Deserialize)]
struct Questions {
    items: Vec<Question>,
    #[serde(rename = "cursor")]
    _cursor: Option<String>,
}

#[derive(Deserialize)]
struct Question {
    seat: String,
    role: String,
    swarm: String,
    node: String,
    #[serde(rename = "generation")]
    _generation: u32,
    item: String,
    message: String,
    #[serde(rename = "asked_at")]
    _asked_at: u64,
    #[serde(flatten)]
    kind: Kind,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Kind {
    Question { schema: Value },
    Approval { options: Vec<String> },
}

enum Form {
    Choices(Vec<Value>),
    Text,
    Number,
    Desktop,
}

impl Question {
    fn form(&self) -> Form {
        let schema = match &self.kind {
            Kind::Question { schema } => schema,
            Kind::Approval { options } => {
                return Form::Choices(options.iter().cloned().map(Value::String).collect());
            }
        };
        if let Some(choices) = schema
            .get("enum")
            .and_then(Value::as_array)
            .filter(|v| !v.is_empty())
        {
            return Form::Choices(choices.clone());
        }
        match schema.get("type").and_then(Value::as_str) {
            Some("boolean") => Form::Choices(vec![json!(true), json!(false)]),
            Some("string") => Form::Text,
            Some("number") => Form::Number,
            _ => Form::Desktop,
        }
    }
}

const FIXTURE: &str = r#"{
  "items": [
    {"seat":"atlas","role":"Executor","swarm":"Mobile","node":"dory","generation":1,
     "item":"q1","kind":"question","message":"Which test suite should I run?",
     "schema":{"type":"string","enum":["Smoke","Full suite"]},"asked_at":1},
    {"seat":"relay","role":"Executor","swarm":"Nodes","node":"mac","generation":1,
     "item":"q2","kind":"question","message":"Keep the emulator running?",
     "schema":{"type":"boolean"},"asked_at":2},
    {"seat":"iris","role":"Oracle","swarm":"Mobile","node":"dory","generation":1,
     "item":"q3","kind":"approval","message":"Approve the local fixture run?",
     "options":["Approve","Decline"],"asked_at":3},
    {"seat":"scribe","role":"Executor","swarm":"Mobile","node":"mac","generation":1,
     "item":"q4","kind":"question","message":"What should the report be called?",
     "schema":{"type":"string"},"asked_at":4},
    {"seat":"meter","role":"Executor","swarm":"Nodes","node":"dory","generation":1,
     "item":"q5","kind":"question","message":"How many samples should I take?",
     "schema":{"type":"number"},"asked_at":5},
    {"seat":"cedar","role":"Architect","swarm":"Mobile","node":"mac","generation":1,
     "item":"q6","kind":"question","message":"Supply the check configuration.",
     "schema":{"type":"object"},"asked_at":6}
  ],
  "cursor":"fixture"
}"#;

pub(crate) struct Inbox {
    questions: Questions,
    answers: HashMap<String, Value>,
    drafts: HashMap<String, String>,
    focus: Option<usize>,
    pub(crate) scroll: f32,
}

#[derive(Clone)]
pub(crate) enum Action {
    Answer(usize, Value),
    Edit(usize),
    Submit(usize),
}

pub(crate) struct Frame {
    pub(crate) draw: DrawList,
    pub(crate) max_scroll: f32,
    clip: Rect,
    controls: Vec<(Rect, Action)>,
}

impl Frame {
    pub(crate) fn hit(&self, point: [f32; 2]) -> Option<Action> {
        let contains = |r: Rect| {
            point[0] >= r.x && point[0] < r.right() && point[1] >= r.y && point[1] < r.bottom()
        };
        if !contains(self.clip) {
            return None;
        }
        self.controls
            .iter()
            .find(|(r, _)| contains(*r))
            .map(|(_, action)| action.clone())
    }
}

struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, label: &str, style: &MeasureStyle) -> f32 {
        text::measure(
            label,
            TextStyle {
                scale: TypeScale::Custom(style.size),
                weight: style.weight,
                mono: style.mono,
                italic: style.italic,
                tracking_em: style.tracking_em,
                ..Default::default()
            },
            None,
        )
        .map_or_else(
            |_| label.chars().count() as f32 * style.size * 0.55,
            |m| m.advance,
        )
    }
}

const GUTTER: f32 = 20.;
const GAP: f32 = 12.;
const ROW: f32 = 152.;
const CONTROL: f32 = 44.;

impl Inbox {
    pub(crate) fn fixture() -> Self {
        Self {
            questions: serde_json::from_str(FIXTURE).expect("valid Questions fixture"),
            answers: HashMap::new(),
            drafts: HashMap::new(),
            focus: None,
            scroll: 0.,
        }
    }

    pub(crate) fn editing(&self) -> bool {
        self.focus.is_some()
    }

    pub(crate) fn type_text(&mut self, text: &str) {
        let Some(index) = self.focus else { return };
        let draft = self
            .drafts
            .entry(self.questions.items[index].item.clone())
            .or_default();
        draft.extend(text.chars().filter(|ch| !ch.is_control()));
    }

    pub(crate) fn backspace(&mut self) {
        if let Some(index) = self.focus {
            self.drafts
                .entry(self.questions.items[index].item.clone())
                .or_default()
                .pop();
        }
    }

    pub(crate) fn submit(&mut self) {
        if let Some(index) = self.focus {
            self.act(Action::Submit(index));
        }
    }

    pub(crate) fn act(&mut self, action: Action) {
        let index = match &action {
            Action::Answer(index, _) | Action::Edit(index) | Action::Submit(index) => *index,
        };
        let question = &self.questions.items[index];
        if self.answers.contains_key(&question.item) {
            return;
        }
        let answer = match action {
            Action::Answer(_, value) => value,
            Action::Edit(_) => {
                self.focus = Some(index);
                // Leave room to move the field above the system keyboard.
                self.scroll = index as f32 * (ROW + GAP);
                return;
            }
            Action::Submit(_) => {
                let draft = self.drafts.get(&question.item).map_or("", String::as_str);
                match question.form() {
                    Form::Text => json!(draft),
                    Form::Number => match serde_json::from_str::<serde_json::Number>(draft) {
                        Ok(number) => Value::Number(number),
                        Err(_) => return,
                    },
                    _ => return,
                }
            }
        };
        self.answers.insert(question.item.clone(), answer);
        self.drafts.remove(&question.item);
        self.focus = None;
    }

    pub(crate) fn layout(&self, screen: Rect, safe: Rect, scale: f32) -> Frame {
        let theme = Theme::Dark;
        let ctx = DrawCtx::new(theme, Density::Comfortable);
        let surface = Color::hex("#151d2a");
        let choice = Color::hex("#263a58");
        let accent = Color::hex("#8fbaff");
        let status = Color::hex("#8edbbb");
        let mut draw = DrawList::default();
        draw.rect(screen, 0., Color::hex("#0c111b"));

        let margin = GUTTER.min(safe.w * 0.1);
        let x = safe.x + margin;
        let width = (safe.w - margin * 2.).max(0.);
        let title_y = safe.y + 14.;
        label(
            &mut draw,
            Rect::new(x, title_y, width, 44.),
            "Inbox",
            34.,
            700,
            ctx.fg(),
        );
        let rule = Rect::new(x, title_y + 55., width, 1.);
        Hairline.paint(
            rule,
            &AtomCtx {
                draw: ctx,
                measure: &Metrics,
                scale,
            },
            &mut draw,
        );
        let y = rule.y + 15.;
        let clip = Rect::new(x, y, width, (safe.bottom() - y - 8.).max(0.));
        let max_scroll = ((ROW + GAP) * self.questions.items.len() as f32 - GAP - clip.h)
            .max(self.focus.map_or(0., |index| index as f32 * (ROW + GAP)));
        let scroll = self.scroll.clamp(0., max_scroll);
        let mut controls = Vec::new();
        if clip.w <= 0. || clip.h <= 0. {
            return Frame {
                draw,
                max_scroll,
                clip,
                controls,
            };
        }
        draw.clip(clip, [0.; 4]);
        for (index, question) in self.questions.items.iter().enumerate() {
            let top = y + index as f32 * (ROW + GAP) - scroll;
            if top + ROW <= clip.y || top >= clip.bottom() {
                continue;
            }
            draw.rect(Rect::new(x, top, width, ROW), 12., surface);
            let inner = Rect::new(x + 14., top + 10., (width - 28.).max(0.), ROW - 20.);
            label(&mut draw, inner, &question.seat, 19., 700, ctx.fg());
            let meta = format!(
                "{}  ·  {}  ·  {}",
                question.role, question.swarm, question.node
            );
            label(
                &mut draw,
                Rect::new(inner.x, top + 36., inner.w, 20.),
                &meta,
                13.,
                500,
                ctx.muted(),
            );
            label(
                &mut draw,
                Rect::new(inner.x, top + 64., inner.w, 25.),
                &question.message,
                16.,
                500,
                ctx.fg(),
            );

            let bar = Rect::new(inner.x, top + 94., inner.w, CONTROL);
            if self.answers.contains_key(&question.item) {
                label(&mut draw, bar, "answered", 14., 650, status);
                continue;
            }
            let mut buttons = Vec::new();
            match question.form() {
                Form::Choices(values) => {
                    for value in values {
                        let title = match &value {
                            Value::Bool(true) => "Yes".to_owned(),
                            Value::Bool(false) => "No".to_owned(),
                            Value::String(label) => label.clone(),
                            _ => value.to_string(),
                        };
                        buttons.push((title, Action::Answer(index, value)));
                    }
                }
                form @ (Form::Text | Form::Number) => {
                    let placeholder = if matches!(form, Form::Number) {
                        "Enter a number"
                    } else {
                        "Enter an answer"
                    };
                    let draft = self.drafts.get(&question.item).filter(|s| !s.is_empty());
                    let title = draft.map_or_else(|| placeholder.into(), String::clone);
                    buttons.push((title, Action::Edit(index)));
                    buttons.push(("Send".into(), Action::Submit(index)));
                }
                Form::Desktop => {
                    label(
                        &mut draw,
                        bar,
                        "Answer on the desktop",
                        14.,
                        500,
                        ctx.muted(),
                    );
                }
            }
            let mut row = Box_::new(
                Axis::Row,
                ctx.density,
                buttons
                    .iter()
                    .map(|(_, action)| {
                        let size = if matches!(action, Action::Submit(_)) {
                            Size::Fixed(68.)
                        } else {
                            Size::Fill
                        };
                        (size, Child::default())
                    })
                    .collect(),
            );
            row.gap = 8.;
            for (bounds, (title, action)) in solve_box(&row, bar, &Metrics).into_iter().zip(buttons)
            {
                let fill = if matches!(action, Action::Edit(i) if self.focus == Some(i)) {
                    Color::hex("#30486b")
                } else {
                    choice
                };
                draw.rect(bounds, 8., fill);
                label(
                    &mut draw,
                    Rect::new(
                        bounds.x + 12.,
                        bounds.y + 11.,
                        (bounds.w - 24.).max(0.),
                        22.,
                    ),
                    &title,
                    15.,
                    600,
                    accent,
                );
                controls.push((bounds, action));
            }
        }
        draw.unclip();
        Frame {
            draw,
            max_scroll,
            clip,
            controls,
        }
    }
}

fn label(draw: &mut DrawList, bounds: Rect, text: &str, size: f32, weight: u16, color: Color) {
    let style = TextStyle {
        scale: TypeScale::Custom(size),
        weight,
        ..Default::default()
    };
    let row = Box_ {
        gap: 0.,
        ..Box_::new(
            Axis::Row,
            Density::Comfortable,
            vec![
                (
                    Size::Text,
                    Child {
                        text: Some((
                            text.to_owned(),
                            MeasureStyle {
                                size,
                                weight,
                                mono: style.mono,
                                italic: style.italic,
                                tracking_em: style.tracking_em,
                            },
                        )),
                        cross: None,
                    },
                ),
                (Size::Fixed(2.), Child::default()),
            ],
        )
    };
    let measured = solve_box(&row, bounds, &Metrics)[0];
    if measured.w > 0. {
        draw.text(
            Rect::new(measured.x, measured.y, measured.w + 2., measured.h),
            text,
            style,
            color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbt_render::chrome::{ChromeRenderer, icons::Assets};
    use bbt_ui_model::draw::Prim;
    use std::path::PathBuf;

    const SCREEN: Rect = Rect::new(0., 0., 390., 844.);
    const SAFE: Rect = Rect::new(0., 47., 390., 763.);
    const QUESTION: &str = "Which test suite should I run?";

    fn labels(frame: &Frame) -> Vec<&str> {
        frame
            .draw
            .primitives
            .iter()
            .filter_map(|p| match p {
                Prim::Text { text, .. } => Some(text.as_ref()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn lays_out_and_paints_inbox_question() {
        let frame = Inbox::fixture().layout(SCREEN, SAFE, 1.);
        frame.draw.validate().unwrap();
        let question = frame
            .draw
            .primitives
            .iter()
            .find_map(|p| match p {
                Prim::Text {
                    origin,
                    text,
                    style,
                    max_width: Some(width),
                    ..
                } if text.as_ref() == QUESTION => Some((*origin, *style, *width)),
                _ => None,
            })
            .expect("question emitted by the kit layout");
        let (origin, style, width) = question;
        let metrics = text::measure(QUESTION, style, None).unwrap();
        assert!(metrics.advance > 180. && width >= metrics.advance);
        assert!(origin[1] >= SAFE.y && origin[1] + style.pixels() <= SAFE.bottom());

        let mut renderer = ChromeRenderer::new(Assets::default()).unwrap();
        let rgba = renderer.render_to_rgba(&frame.draw, 390, 844, 1.).unwrap();
        let mut without_question = frame.draw;
        without_question
            .primitives
            .retain(|p| !matches!(p, Prim::Text { text, .. } if text.as_ref() == QUESTION));
        let reference = renderer
            .render_to_rgba(&without_question, 390, 844, 1.)
            .unwrap();
        let changed = rgba
            .chunks_exact(4)
            .zip(reference.chunks_exact(4))
            .enumerate()
            .filter(|(_, (painted, empty))| painted != empty)
            .map(|(i, _)| i);
        let mut glyph_pixels = 0;
        for pixel in changed {
            let x = (pixel % 390) as f32;
            let y = (pixel / 390) as f32;
            assert!(x >= origin[0] && x <= origin[0] + width + 1.);
            assert!(y >= origin[1] && y <= origin[1] + style.pixels() * 1.4 + 1.);
            glyph_pixels += 1;
        }
        assert!(
            glyph_pixels > 200,
            "no visible question glyphs: {glyph_pixels}"
        );
        let background = &rgba[..4];
        assert!(rgba.chunks_exact(4).filter(|px| *px != background).count() > 50_000);

        let output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/smoke/host.png");
        std::fs::create_dir_all(output.parent().unwrap()).unwrap();
        bbt_render::write_png(&output, &rgba, 390, 844).unwrap();
        println!(
            "host Inbox: {glyph_pixels} question glyph pixels at {origin:?}; {}",
            output.display()
        );
    }

    #[test]
    fn accepts_visible_choices_once_and_clips_scroll_hits() {
        let mut inbox = Inbox::fixture();
        let frame = inbox.layout(SCREEN, SAFE, 1.);
        assert!(frame.max_scroll > 0.);
        let hit = frame.hit([80., 247.]).expect("the first answer button");
        inbox.act(hit);
        inbox.act(Action::Answer(0, json!("Full suite")));
        assert_eq!(inbox.answers["q1"], "Smoke");
        let updated = inbox.layout(SCREEN, SAFE, 1.);
        assert!(labels(&updated).contains(&"answered"));
        assert!(updated.hit([80., 247.]).is_none());
        assert!(labels(&updated).contains(&"Yes"));
        assert!(labels(&updated).contains(&"No"));
        assert!(labels(&updated).contains(&"Approve"));
        inbox.scroll = 10_000.;
        let bottom = inbox.layout(SCREEN, SAFE, 1.);
        assert!(labels(&bottom).contains(&"Answer on the desktop"));
        assert!(bottom.hit([80., bottom.clip.y - 1.]).is_none());
    }

    #[test]
    fn records_text_and_numbers_as_typed_values() {
        let mut inbox = Inbox::fixture();
        inbox.act(Action::Edit(3));
        let editing = inbox.layout(SCREEN, SAFE, 1.);
        assert!(matches!(editing.hit([80., 247.]), Some(Action::Edit(3))));
        inbox.type_text("Inbox e2e!");
        inbox.backspace();
        inbox.submit();
        assert_eq!(inbox.answers["q4"], "Inbox e2e");
        inbox.act(Action::Edit(4));
        inbox.type_text("invalid");
        inbox.submit();
        assert!(!inbox.answers.contains_key("q5") && inbox.editing());
        for _ in 0.."invalid".len() {
            inbox.backspace();
        }
        inbox.type_text("-2.5");
        inbox.submit();
        assert_eq!(inbox.answers["q5"], json!(-2.5));
        assert!(!inbox.editing());
    }
}
