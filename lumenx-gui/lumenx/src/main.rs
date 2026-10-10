//! LUMENX — native desktop GUI (egui/eframe).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod game;

use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, RichText, Sense, Stroke, StrokeKind, Vec2,
};
use game::{Card, Game, Phase, COLOR_NAMES, GAMES_PER_MATCH, HAND_SIZE, HUMAN, MAX_PLAYERS, MIN_PLAYERS};

const AI_DELAY: Duration = Duration::from_millis(700);
const TRICK_DISPLAY: Duration = Duration::from_millis(1800);

const CARD_COLORS: [Color32; 5] = [
    Color32::from_rgb(0xE0, 0xB0, 0x00), // Yellow (trump)
    Color32::from_rgb(0x2E, 0x9E, 0x44), // Green
    Color32::from_rgb(0x25, 0x63, 0xEB), // Blue
    Color32::from_rgb(0xDC, 0x26, 0x26), // Red
    Color32::from_rgb(0x93, 0x33, 0xEA), // Purple
];

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("LumenX")
            .with_app_id("lumenx")
            .with_inner_size([960.0, 720.0])
            .with_min_inner_size([760.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "LumenX",
        options,
        Box::new(|cc| Ok(Box::new(LumenApp::new(cc)))),
    )
}

struct LumenApp {
    game: Game,
    player_setting: usize,
    guess_input: u32,
    hide_own_values: bool,
    show_rules: bool,
    /// Time of the last state change; drives AI moves and trick display.
    last_event: Instant,
}

impl LumenApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        LumenApp {
            game: Game::new(3),
            player_setting: 3,
            guess_input: 2,
            hide_own_values: true,
            show_rules: false,
            last_event: Instant::now(),
        }
    }

    fn new_match(&mut self) {
        self.game = Game::new(self.player_setting);
        self.guess_input = 2;
        self.last_event = Instant::now();
    }

    fn touch(&mut self) {
        self.last_event = Instant::now();
    }

    /// Advances computer moves / trick collection based on elapsed time.
    fn tick(&mut self, ctx: &egui::Context) {
        let elapsed = self.last_event.elapsed();
        match self.game.phase {
            Phase::Playing if self.game.turn != HUMAN => {
                if elapsed >= AI_DELAY {
                    let p = self.game.turn;
                    let idx = self.game.ai_choice(p);
                    self.game.play(p, idx);
                    self.touch();
                }
                ctx.request_repaint_after(AI_DELAY.saturating_sub(elapsed));
            }
            Phase::TrickComplete => {
                if elapsed >= TRICK_DISPLAY {
                    self.game.collect_trick();
                    self.touch();
                }
                ctx.request_repaint_after(Duration::from_millis(30));
            }
            _ => {}
        }
    }

    fn player_name(p: usize) -> String {
        if p == HUMAN { "Du".into() } else { format!("Spieler {p}") }
    }

    fn status_text(&self) -> String {
        let g = &self.game;
        match g.phase {
            Phase::Guessing => "Gib deinen Tipp ab.".into(),
            Phase::Playing if g.turn == HUMAN => match g.led_color() {
                None => "Du spielst aus — wähle eine beliebige Karte.".into(),
                Some(c) if g.hands[HUMAN].iter().any(|x| x.color == c) => {
                    format!("Du bist dran — bediene {}.", COLOR_NAMES[c as usize])
                }
                Some(_) => "Du bist dran — du kannst nicht bedienen, freie Wahl.".into(),
            },
            Phase::Playing => format!("{} ist am Zug …", Self::player_name(g.turn)),
            Phase::TrickComplete => {
                let w = g.trick_winner().unwrap();
                if w == HUMAN { "Du gewinnst den Stich!".into() }
                else { format!("{} gewinnt den Stich.", Self::player_name(w)) }
            }
            Phase::GameOver => "Spiel beendet.".into(),
            Phase::MatchOver => "Match beendet.".into(),
        }
    }

    // ---------------------------------------------------------------- panels

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("LUMENX").font(FontId::proportional(26.0)).strong());
            ui.separator();
            ui.label(format!(
                "Spiel {}/{}  ·  {} Punkte pro Stich",
                self.game.game_no + 1,
                GAMES_PER_MATCH,
                self.game.multiplier()
            ));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Neues Match").clicked() {
                    self.new_match();
                }
                ui.add(
                    egui::DragValue::new(&mut self.player_setting)
                        .range(MIN_PLAYERS..=MAX_PLAYERS)
                        .suffix(" Spieler"),
                )
                .on_hover_text("Gilt ab dem nächsten Match");
                if ui.button("Spielregeln").clicked() {
                    self.show_rules = true;
                }
                ui.checkbox(&mut self.hide_own_values, "Eigene Werte verdeckt")
                    .on_hover_text("Original-Regel: du siehst nur die Farben deiner Karten");
            });
        });
    }

    fn opponents(&self, ui: &mut egui::Ui) {
        let g = &self.game;
        for p in 1..g.players {
            ui.horizontal(|ui| {
                ui.allocate_ui(Vec2::new(150.0, 70.0), |ui| {
                    ui.vertical(|ui| {
                        let mut name = RichText::new(Self::player_name(p)).strong();
                        if g.phase == Phase::Playing && g.turn == p {
                            name = name.color(Color32::from_rgb(0x25, 0x63, 0xEB));
                        }
                        ui.label(name);
                        ui.label(self.stats_line(p));
                    });
                });
                for card in &g.hands[p] {
                    draw_card(ui, Some(*card), Vec2::new(36.0, 52.0), CardStyle::Normal, false);
                }
            });
            ui.add_space(6.0);
        }
    }

    fn stats_line(&self, p: usize) -> String {
        let g = &self.game;
        let guess = if p == HUMAN && g.phase == Phase::Guessing {
            "–".to_string()
        } else {
            g.guesses[p].to_string()
        };
        format!("Tipp {guess} · Stiche {} · Punkte {}", g.tricks[p], g.totals[p])
    }

    fn table(&self, ui: &mut egui::Ui) {
        let g = &self.game;
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("Tisch").color(Color32::GRAY));
            ui.add_space(4.0);
            let winning = if g.phase == Phase::TrickComplete { g.winning_index() } else { None };
            let n = g.trick.len().max(1) as f32;
            let width = n * 76.0;
            ui.allocate_ui(Vec2::new(width, 120.0), |ui| {
                ui.horizontal(|ui| {
                    if g.trick.is_empty() {
                        ui.add_space(width / 2.0 - 30.0);
                        ui.label(RichText::new("(leer)").color(Color32::GRAY));
                    }
                    for (i, (p, card)) in g.trick.iter().enumerate() {
                        ui.vertical(|ui| {
                            let style = if Some(i) == winning { CardStyle::Winner } else { CardStyle::Normal };
                            draw_card(ui, Some(*card), Vec2::new(60.0, 86.0), style, false);
                            ui.label(Self::player_name(*p));
                        });
                    }
                });
            });
            if g.phase == Phase::TrickComplete {
                let frac = (self.last_event.elapsed().as_secs_f32() / TRICK_DISPLAY.as_secs_f32()).min(1.0);
                ui.add(egui::ProgressBar::new(frac).desired_width(260.0));
            }
        });
    }

    fn hand(&mut self, ui: &mut egui::Ui) {
        let my_turn = self.game.phase == Phase::Playing && self.game.turn == HUMAN;
        let legal = self.game.legal_moves(HUMAN);
        let mut clicked = None;

        ui.horizontal(|ui| {
            ui.label(RichText::new("Deine Karten").strong());
            ui.label(self.stats_line(HUMAN));
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            for (i, card) in self.game.hands[HUMAN].iter().enumerate() {
                let shown = if self.hide_own_values { None } else { Some(*card) };
                let style = match (my_turn, legal[i]) {
                    (true, true) => CardStyle::Playable,
                    (true, false) => CardStyle::Disabled,
                    _ => CardStyle::Normal,
                };
                let resp = draw_card_colored(ui, shown, card.color, Vec2::new(60.0, 86.0), style, my_turn && legal[i]);
                if resp.clicked() {
                    clicked = Some(i);
                }
            }
        });

        if let Some(i) = clicked {
            if self.game.play(HUMAN, i) {
                self.touch();
            }
        }
    }

    // ---------------------------------------------------------------- modals

    fn guess_modal(&mut self, ctx: &egui::Context) {
        egui::Modal::new(egui::Id::new("guess")).show(ctx, |ui| {
            ui.set_width(300.0);
            ui.heading(format!("Spiel {} — dein Tipp", self.game.game_no + 1));
            ui.label("Wie viele Stiche wirst du machen?");
            ui.add_space(8.0);
            ui.add(egui::Slider::new(&mut self.guess_input, 0..=HAND_SIZE as u32).text("Stiche"));
            ui.add_space(8.0);
            let starter = Self::player_name(self.game.leader);
            ui.label(RichText::new(format!("{starter} spielt aus.")).color(Color32::GRAY));
            ui.add_space(8.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("OK").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.game.set_human_guess(self.guess_input);
                    self.touch();
                }
            });
        });
    }

    fn results_modal(&mut self, ctx: &egui::Context) {
        let g = &self.game;
        let match_over = g.phase == Phase::MatchOver;
        let mut next = false;
        let mut restart = false;

        egui::Modal::new(egui::Id::new("results")).show(ctx, |ui| {
            ui.set_width(380.0);
            if match_over {
                let winners: Vec<String> = g.match_winners().into_iter().map(Self::player_name).collect();
                let title = if winners == ["Du"] { "Du hast gewonnen!".to_string() }
                    else { format!("Sieger: {}", winners.join(", ")) };
                ui.heading(title);
            } else {
                ui.heading(format!("Ergebnis Spiel {}", g.game_no + 1));
            }
            ui.add_space(8.0);

            egui::Grid::new("scores").striped(true).spacing([18.0, 6.0]).show(ui, |ui| {
                for h in ["Spieler", "Tipp", "Stiche", "Punkte", "Gesamt"] {
                    ui.label(RichText::new(h).strong());
                }
                ui.end_row();
                let mut order: Vec<usize> = (0..g.players).collect();
                order.sort_by_key(|&p| -g.totals[p]);
                for p in order {
                    ui.label(Self::player_name(p));
                    ui.label(g.guesses[p].to_string());
                    ui.label(g.tricks[p].to_string());
                    let pts = g.last_scores[p];
                    let col = if g.guesses[p] == g.tricks[p] { Color32::from_rgb(0x2E, 0x9E, 0x44) } else { Color32::from_rgb(0xDC, 0x26, 0x26) };
                    ui.label(RichText::new(format!("{pts:+}")).color(col));
                    ui.label(RichText::new(g.totals[p].to_string()).strong());
                    ui.end_row();
                }
            });

            ui.add_space(12.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if match_over {
                    restart = ui.button("Neues Match").clicked();
                } else {
                    let label = if g.game_no + 1 == GAMES_PER_MATCH { "Endergebnis" } else { "Nächstes Spiel" };
                    next = ui.button(label).clicked();
                }
            });
        });

        if next {
            self.game.next_game();
            self.touch();
        }
        if restart {
            self.new_match();
        }
    }

    fn rules_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Spielregeln")
            .open(&mut self.show_rules)
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.label("• 60 Karten: 5 Farben (Gelb, Grün, Blau, Rot, Lila) mit Werten 1–12. Jeder bekommt 10 Karten.");
                ui.label("• Vor jedem Spiel tippt jeder, wie viele Stiche er macht. Im Original siehst du dabei nur die Farben deiner Karten.");
                ui.label("• Wer ausspielt, wählt eine beliebige Karte. Alle anderen müssen die Farbe bedienen, wenn sie können.");
                ui.label("• Gelb ist Trumpf und schlägt jede andere Farbe. Sonst gewinnt die höchste Karte der ausgespielten Farbe.");
                ui.label("• Der Stichgewinner spielt den nächsten Stich aus.");
                ui.label("• Ein Match hat 4 Spiele. Jeder Stich zählt 10 / 20 / 30 / 40 Punkte, jeder Stich Abweichung vom Tipp kostet 5 Punkte.");
            });
    }
}

impl eframe::App for LumenApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.tick(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        egui::Panel::top("top").show(ui, |ui| {
            ui.add_space(6.0);
            self.top_bar(ui);
            ui.add_space(6.0);
        });

        egui::Panel::bottom("hand").show(ui, |ui| {
            ui.add_space(8.0);
            ui.label(RichText::new(self.status_text()).font(FontId::proportional(18.0)));
            ui.add_space(6.0);
            self.hand(ui);
            ui.add_space(10.0);
        });

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.opponents(ui);
                ui.separator();
                ui.add_space(8.0);
                self.table(ui);
            });
        });

        match self.game.phase {
            Phase::Guessing => self.guess_modal(&ctx),
            Phase::GameOver | Phase::MatchOver => self.results_modal(&ctx),
            _ => {}
        }
        if self.show_rules {
            self.rules_window(&ctx);
        }
    }
}

// -------------------------------------------------------------------- cards

#[derive(Clone, Copy, PartialEq)]
enum CardStyle {
    Normal,
    Playable,
    Disabled,
    Winner,
}

fn draw_card(ui: &mut egui::Ui, card: Option<Card>, size: Vec2, style: CardStyle, clickable: bool) -> egui::Response {
    let color = card.map(|c| c.color).unwrap_or(0);
    draw_card_colored(ui, card, color, size, style, clickable)
}

/// Draws a card. `card == None` shows only the color (value hidden).
fn draw_card_colored(
    ui: &mut egui::Ui,
    card: Option<Card>,
    color: u8,
    size: Vec2,
    style: CardStyle,
    clickable: bool,
) -> egui::Response {
    let sense = if clickable { Sense::click() } else { Sense::hover() };
    let (rect, mut resp) = ui.allocate_exact_size(size, sense);
    if clickable {
        resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    }

    let base = CARD_COLORS[color as usize];
    let lifted = clickable && resp.hovered();
    let rect = if lifted { rect.translate(Vec2::new(0.0, -6.0)) } else { rect };
    let radius = CornerRadius::same(6);
    let painter = ui.painter();

    let (fill, stroke_w, alpha) = match style {
        CardStyle::Disabled => (Color32::from_gray(240), 2.0, 90),
        CardStyle::Winner => (base.gamma_multiply(0.15), 4.0, 255),
        CardStyle::Playable => (Color32::WHITE, 3.0, 255),
        CardStyle::Normal => (Color32::WHITE, 2.0, 255),
    };
    let stroke_col = Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), alpha);

    painter.rect_filled(rect, radius, fill);
    painter.rect_stroke(rect, radius, Stroke::new(stroke_w, stroke_col), StrokeKind::Inside);
    if style == CardStyle::Winner {
        painter.rect_stroke(rect.expand(3.0), CornerRadius::same(8), Stroke::new(2.0, Color32::BLACK), StrokeKind::Outside);
    }

    match card {
        Some(c) => {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                c.value.to_string(),
                FontId::proportional(size.y * 0.32),
                stroke_col,
            );
        }
        None => {
            // Face-down: colored stripes instead of a value.
            let inner = rect.shrink(size.x * 0.22);
            painter.rect_filled(inner, CornerRadius::same(3), stroke_col.gamma_multiply(0.35));
        }
    }
    resp
}
