#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;

const ROWS: [[&str; 4]; 5] = [
    ["C", "Del", "(", ")"],
    ["7", "8", "9", "÷"],
    ["4", "5", "6", "×"],
    ["1", "2", "3", "-"],
    ["0", ".", "=", "+"],
];

const MAX_LEN: usize = 64;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([320.0, 460.0])
            .with_min_inner_size([260.0, 380.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Calculator",
        options,
        Box::new(|_| Ok(Box::<Calc>::default())),
    )
}

#[derive(Default)]
struct Calc {
    expr: String,
    note: String,
    fresh: bool, // last key was "=", so typing a number starts over
}

impl Calc {
    fn press(&mut self, key: &str) {
        match key {
            "C" => {
                self.expr.clear();
                self.note.clear();
            }
            "Del" => {
                self.expr.pop();
            }
            "=" => {
                match eval(&self.expr) {
                    Ok(v) => {
                        self.note = format!("{} =", self.expr);
                        self.expr = format_num(v);
                        self.fresh = true;
                    }
                    Err(e) => self.note = e,
                }
                return;
            }
            _ => {
                if self.fresh
                    && key.starts_with(|c: char| c.is_ascii_digit() || c == '.' || c == '(')
                {
                    self.expr.clear();
                }
                if self.expr.chars().count() < MAX_LEN {
                    self.expr.push_str(key);
                }
            }
        }
        self.fresh = false;
    }
}

fn key_for_event(e: &egui::Event) -> Option<&'static str> {
    match e {
        egui::Event::Text(t) => {
            let mut chars = t.chars();
            let c = match (chars.next()?, chars.next()) {
                ('*', None) => '×',
                ('/', None) => '÷',
                (c, None) => c,
                _ => return None,
            };
            ROWS.iter().flatten().copied().find(|k| k.chars().eq([c]))
        }
        egui::Event::Key {
            key, pressed: true, ..
        } => match key {
            egui::Key::Enter => Some("="),
            egui::Key::Backspace => Some("Del"),
            egui::Key::Escape => Some("C"),
            _ => None,
        },
        _ => None,
    }
}

impl eframe::App for Calc {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let keys: Vec<&str> = ui.input(|i| i.events.iter().filter_map(key_for_event).collect());
        // Otherwise Enter would also click whichever button has focus.
        ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        for key in keys {
            self.press(key);
        }

        egui::CentralPanel::default().show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                ui.label(egui::RichText::new(&self.note).size(16.0).weak());
                let shown = if self.expr.is_empty() {
                    "0"
                } else {
                    &self.expr
                };
                ui.label(egui::RichText::new(shown).size(36.0).strong());
            });
            ui.add_space(8.0);

            let gap = ui.spacing().item_spacing;
            let w = (ui.available_width() - 3.0 * gap.x) / 4.0;
            let h = (ui.available_height() - 4.0 * gap.y) / 5.0;
            for row in ROWS {
                ui.horizontal(|ui| {
                    for key in row {
                        let mut btn = egui::Button::new(egui::RichText::new(key).size(22.0))
                            .min_size(egui::vec2(w, h));
                        if key == "=" {
                            btn = btn.fill(ui.visuals().selection.bg_fill);
                        }
                        if ui.add(btn).clicked() {
                            self.press(key);
                        }
                    }
                });
            }
        });
    }
}

/// Evaluates `+ - × ÷` (or `* /`), parentheses and unary minus, with normal precedence.
fn eval(s: &str) -> Result<f64, String> {
    let mut p = Parser {
        chars: s.chars().filter(|c| !c.is_whitespace()).collect(),
        pos: 0,
    };
    let v = p.expr()?;
    if let Some(c) = p.peek() {
        return Err(format!("Unexpected '{c}'"));
    }
    if !v.is_finite() {
        return Err("Overflow".into());
    }
    Ok(v)
}

// ponytail: fixed 10 decimal places, so anything under 1e-10 shows as 0; use scientific notation if that matters
fn format_num(x: f64) -> String {
    let s = format!("{x:.10}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn eat(&mut self, ops: &[char]) -> Option<char> {
        let c = self.peek().filter(|c| ops.contains(c))?;
        self.pos += 1;
        Some(c)
    }

    fn expr(&mut self) -> Result<f64, String> {
        let mut v = self.term()?;
        while let Some(op) = self.eat(&['+', '-']) {
            let r = self.term()?;
            v = if op == '+' { v + r } else { v - r };
        }
        Ok(v)
    }

    fn term(&mut self) -> Result<f64, String> {
        let mut v = self.factor()?;
        while let Some(op) = self.eat(&['*', '×', '/', '÷']) {
            let r = self.factor()?;
            if matches!(op, '/' | '÷') {
                if r == 0.0 {
                    return Err("Can't divide by zero".into());
                }
                v /= r;
            } else {
                v *= r;
            }
        }
        Ok(v)
    }

    fn factor(&mut self) -> Result<f64, String> {
        if self.eat(&['-']).is_some() {
            return Ok(-self.factor()?);
        }
        if self.eat(&['(']).is_some() {
            let v = self.expr()?;
            return self
                .eat(&[')'])
                .map(|_| v)
                .ok_or_else(|| "Missing ')'".into());
        }
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '.') {
            self.pos += 1;
        }
        let num: String = self.chars[start..self.pos].iter().collect();
        num.parse()
            .map_err(|_| match (num.is_empty(), self.peek()) {
                (false, _) => format!("Bad number '{num}'"),
                (true, Some(c)) => format!("Unexpected '{c}'"),
                (true, None) => "Incomplete expression".into(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates() {
        assert_eq!(eval("2+3×4"), Ok(14.0));
        assert_eq!(eval("(2+3)*4"), Ok(20.0));
        assert_eq!(eval("10-4-3"), Ok(3.0));
        assert_eq!(eval("8÷4÷2"), Ok(1.0));
        assert_eq!(eval("-(3--2)"), Ok(-5.0));
        assert_eq!(eval(".5+5."), Ok(5.5));
        assert_eq!(eval("1/0"), Err("Can't divide by zero".into()));
        assert_eq!(eval("2+"), Err("Incomplete expression".into()));
        assert_eq!(eval("(2"), Err("Missing ')'".into()));
        assert_eq!(eval("2)"), Err("Unexpected ')'".into()));
        assert_eq!(eval("1.2.3"), Err("Bad number '1.2.3'".into()));
        assert_eq!(eval(""), Err("Incomplete expression".into()));
    }

    #[test]
    fn formats() {
        assert_eq!(format_num(eval("0.1+0.2").unwrap()), "0.3");
        assert_eq!(format_num(14.0), "14");
        assert_eq!(format_num(-2.5), "-2.5");
        assert_eq!(format_num(-0.0), "0");
    }

    #[test]
    fn result_feeds_next_calculation() {
        let mut c = Calc::default();
        for k in ["7", "÷", "2", "=", "×", "2", "="] {
            c.press(k);
        }
        assert_eq!(c.expr, "7");
        c.press("5"); // a digit after "=" starts fresh
        assert_eq!(c.expr, "5");
    }
}
