//! LUMENX game rules — UI-independent, ported from index.html / server2.js.
//!
//! Rules:
//! - 60 cards: 5 colors (Yellow, Green, Blue, Red, Purple) × values 1–12.
//! - Each player gets 10 cards. Player 0 is the human.
//! - Before each game every player guesses how many tricks they will take.
//! - The leader plays any card; the others must follow the led color if they
//!   can. Yellow is trump: it beats every other color.
//! - The winner of a trick leads the next one.
//! - A match has 4 games. Points per trick: 10 / 20 / 30 / 40 by game.
//!   points = tricks × multiplier − |guess − tricks| × 5

use rand::{seq::SliceRandom, Rng};

pub const COLORS: u8 = 5;
pub const VALUES: u8 = 12;
pub const HAND_SIZE: usize = 10;
pub const GAMES_PER_MATCH: usize = 4;
pub const MIN_PLAYERS: usize = 3;
pub const MAX_PLAYERS: usize = 6; // 60 cards / 10
pub const HUMAN: usize = 0;
pub const YELLOW: u8 = 0;

pub const COLOR_NAMES: [&str; 5] = ["Gelb", "Grün", "Blau", "Rot", "Lila"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Card {
    pub value: u8,
    pub color: u8,
}

impl Card {
    /// Port of `isBiggerThan(card1, card2)`: does `self` beat `other`
    /// (`other` being the card currently winning the trick)?
    pub fn beats(&self, other: &Card) -> bool {
        if self.color == other.color {
            self.value > other.value
        } else {
            self.color == YELLOW
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Waiting for the human's guess.
    Guessing,
    /// A trick is in progress; `turn` says who plays next.
    Playing,
    /// All players have played; trick is shown before it is collected.
    TrickComplete,
    /// All 10 tricks played; scores of this game are in `last_scores`.
    GameOver,
    /// All 4 games played.
    MatchOver,
}

#[derive(Debug, Clone)]
pub struct Game {
    pub players: usize,
    /// 0-based index of the current game within the match.
    pub game_no: usize,
    pub hands: Vec<Vec<Card>>,
    pub guesses: Vec<u32>,
    pub tricks: Vec<u32>,
    pub totals: Vec<i32>,
    pub last_scores: Vec<i32>,
    /// Cards on the table: (player, card), in play order.
    pub trick: Vec<(usize, Card)>,
    pub leader: usize,
    pub turn: usize,
    pub phase: Phase,
}

impl Game {
    pub fn new(players: usize) -> Self {
        let players = players.clamp(MIN_PLAYERS, MAX_PLAYERS);
        let mut g = Game {
            players,
            game_no: 0,
            hands: vec![],
            guesses: vec![0; players],
            tricks: vec![0; players],
            totals: vec![0; players],
            last_scores: vec![0; players],
            trick: vec![],
            leader: 0,
            turn: 0,
            phase: Phase::Guessing,
        };
        g.deal();
        g
    }

    pub fn multiplier(&self) -> i32 {
        10 * (self.game_no as i32 + 1)
    }

    fn deal(&mut self) {
        let mut rng = rand::rng();
        let mut deck: Vec<Card> = (0..COLORS)
            .flat_map(|color| (1..=VALUES).map(move |value| Card { value, color }))
            .collect();
        deck.shuffle(&mut rng);

        self.hands = deck
            .chunks(HAND_SIZE)
            .take(self.players)
            .map(|h| {
                let mut h = h.to_vec();
                h.sort_by_key(|c| (c.color, c.value));
                h
            })
            .collect();

        self.tricks = vec![0; self.players];
        self.last_scores = vec![0; self.players];
        self.trick.clear();
        // Computer players guess randomly 0–5, like the original.
        self.guesses = (0..self.players)
            .map(|p| if p == HUMAN { 0 } else { rng.random_range(0..=5) })
            .collect();
        self.leader = rng.random_range(0..self.players);
        self.turn = self.leader;
        self.phase = Phase::Guessing;
    }

    pub fn set_human_guess(&mut self, guess: u32) {
        assert_eq!(self.phase, Phase::Guessing);
        self.guesses[HUMAN] = guess.min(HAND_SIZE as u32);
        self.phase = Phase::Playing;
    }

    pub fn led_color(&self) -> Option<u8> {
        self.trick.first().map(|(_, c)| c.color)
    }

    /// Which cards of `player` may legally be played right now.
    pub fn legal_moves(&self, player: usize) -> Vec<bool> {
        let hand = &self.hands[player];
        match self.led_color() {
            Some(led) if hand.iter().any(|c| c.color == led) => {
                hand.iter().map(|c| c.color == led).collect()
            }
            _ => vec![true; hand.len()],
        }
    }

    /// Plays card `idx` from `player`'s hand. Returns false if not allowed.
    pub fn play(&mut self, player: usize, idx: usize) -> bool {
        if self.phase != Phase::Playing
            || player != self.turn
            || !self.legal_moves(player).get(idx).copied().unwrap_or(false)
        {
            return false;
        }
        let card = self.hands[player].remove(idx);
        self.trick.push((player, card));
        if self.trick.len() == self.players {
            self.phase = Phase::TrickComplete;
        } else {
            self.turn = (self.turn + 1) % self.players;
        }
        true
    }

    /// Computer strategy from the original: follow the led color (lowest
    /// card), otherwise play a yellow (lowest), otherwise a random card.
    /// As leader, play a random card.
    pub fn ai_choice(&self, player: usize) -> usize {
        let hand = &self.hands[player];
        let mut rng = rand::rng();
        if let Some(led) = self.led_color() {
            if let Some(i) = hand.iter().position(|c| c.color == led) {
                return i;
            }
            if let Some(i) = hand.iter().position(|c| c.color == YELLOW) {
                return i;
            }
        }
        rng.random_range(0..hand.len())
    }

    /// Index into `trick` of the currently winning card.
    pub fn winning_index(&self) -> Option<usize> {
        let mut best = 0;
        for (i, (_, card)) in self.trick.iter().enumerate().skip(1) {
            if card.beats(&self.trick[best].1) {
                best = i;
            }
        }
        (!self.trick.is_empty()).then_some(best)
    }

    pub fn trick_winner(&self) -> Option<usize> {
        self.winning_index().map(|i| self.trick[i].0)
    }

    /// Collects a completed trick; the winner leads next.
    pub fn collect_trick(&mut self) {
        assert_eq!(self.phase, Phase::TrickComplete);
        let winner = self.trick_winner().expect("trick is not empty");
        self.tricks[winner] += 1;
        self.trick.clear();
        self.leader = winner;
        self.turn = winner;

        if self.hands.iter().all(|h| h.is_empty()) {
            self.score();
            self.phase = Phase::GameOver;
        } else {
            self.phase = Phase::Playing;
        }
    }

    fn score(&mut self) {
        let m = self.multiplier();
        for p in 0..self.players {
            let tricks = self.tricks[p] as i32;
            let diff = (tricks - self.guesses[p] as i32).abs();
            let points = tricks * m - diff * 5;
            self.last_scores[p] = points;
            self.totals[p] += points;
        }
    }

    pub fn next_game(&mut self) {
        assert_eq!(self.phase, Phase::GameOver);
        self.game_no += 1;
        if self.game_no >= GAMES_PER_MATCH {
            self.game_no = GAMES_PER_MATCH - 1;
            self.phase = Phase::MatchOver;
        } else {
            self.deal();
        }
    }

    /// Players with the highest total (several on a tie).
    pub fn match_winners(&self) -> Vec<usize> {
        let max = *self.totals.iter().max().unwrap_or(&0);
        (0..self.players).filter(|&p| self.totals[p] == max).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn c(value: u8, color: u8) -> Card {
        Card { value, color }
    }

    #[test]
    fn deal_is_unique_and_sorted() {
        for n in MIN_PLAYERS..=MAX_PLAYERS {
            let g = Game::new(n);
            let all: Vec<_> = g.hands.iter().flatten().collect();
            assert_eq!(all.len(), n * HAND_SIZE);
            let set: HashSet<_> = all.iter().map(|c| (c.value, c.color)).collect();
            assert_eq!(set.len(), all.len());
            for h in &g.hands {
                assert!(h.windows(2).all(|w| (w[0].color, w[0].value) < (w[1].color, w[1].value)));
            }
        }
    }

    #[test]
    fn trump_and_follow_rules() {
        assert!(c(1, YELLOW).beats(&c(12, 3)));
        assert!(!c(12, 3).beats(&c(1, YELLOW)));
        assert!(c(9, 3).beats(&c(8, 3)));
        assert!(!c(12, 2).beats(&c(1, 3))); // off-color, not trump
        assert!(c(5, YELLOW).beats(&c(4, YELLOW)));
    }

    #[test]
    fn must_follow_suit() {
        let mut g = Game::new(3);
        g.set_human_guess(2);
        g.hands[0] = vec![c(3, 1), c(7, 3)];
        g.hands[1] = vec![c(5, 3), c(1, 2)];
        g.hands[2] = vec![c(2, 2), c(9, 4)];
        g.turn = 0;
        assert!(g.play(0, 1)); // red 7
        assert_eq!(g.legal_moves(1), vec![true, false]);
        assert!(!g.play(1, 1));
        assert!(g.play(1, 0));
        assert_eq!(g.legal_moves(2), vec![true, true]); // no red → free
        assert!(g.play(2, 0));
        assert_eq!(g.phase, Phase::TrickComplete);
        assert_eq!(g.trick_winner(), Some(0));
    }

    #[test]
    fn full_match_with_ai_only() {
        let mut g = Game::new(4);
        for _ in 0..GAMES_PER_MATCH {
            g.set_human_guess(3);
            while g.phase != Phase::GameOver {
                match g.phase {
                    Phase::Playing => {
                        let p = g.turn;
                        let i = g.ai_choice(p);
                        assert!(g.play(p, i), "AI chose an illegal card");
                    }
                    Phase::TrickComplete => g.collect_trick(),
                    _ => unreachable!(),
                }
            }
            assert_eq!(g.tricks.iter().sum::<u32>(), HAND_SIZE as u32);
            g.next_game();
        }
        assert_eq!(g.phase, Phase::MatchOver);
    }

    #[test]
    fn scoring_formula() {
        let mut g = Game::new(3);
        g.game_no = 1; // multiplier 20
        g.guesses = vec![3, 0, 5];
        g.tricks = vec![3, 0, 2];
        g.score();
        assert_eq!(g.last_scores, vec![60, 0, 40 - 15]);
    }
}
