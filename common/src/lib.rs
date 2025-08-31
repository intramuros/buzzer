use std::collections::{HashMap, VecDeque};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod message;
pub use message::*;

pub static HOST: &'static str = "HOST";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Actor {
    Host { id: Uuid },
    Player { id: Uuid, name: String },
}

impl Actor {
    pub fn name(&self) -> &str {
        match self {
            Self::Host { .. } => HOST,
            Self::Player { name, .. } => name,
        }
    }

    pub fn id(&self) -> Uuid {
        match self {
            Self::Host { id } => *id,
            Self::Player { id, .. } => *id,
        }
    }
}

// Using a special DashMap type that works with Dioxus signals
type PlayersMap = DashMap<Uuid, Actor>;

#[derive(Debug, Clone, Default)]
pub struct GameState {
    pub host_id: Uuid,
    pub globally_locked: bool,
    pub buzzer_order: VecDeque<(Uuid, String)>,
    pub players: PlayersMap,
    pub scores: HashMap<Uuid, i32>,
    pub player_join_order: Vec<Uuid>,
    pub time_limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameStateJson {
    host_id: Uuid,
    locked: bool,
    buzzer_order: VecDeque<(Uuid, String)>,
    players: HashMap<Uuid, Actor>,
    scores: HashMap<Uuid, i32>,
    player_join_order: Vec<Uuid>,
    time_limit: Option<u32>,
}

impl GameState {
    pub fn to_json(&self) -> GameStateJson {
        GameStateJson {
            host_id: self.host_id,
            locked: self.globally_locked,
            buzzer_order: self.buzzer_order.clone(),
            players: self.players.clone().into_iter().collect(),
            scores: self.scores.clone(),
            player_join_order: self.player_join_order.clone(),
            time_limit: self.time_limit,
        }
    }
}

impl From<GameStateJson> for GameState {
    fn from(json: GameStateJson) -> Self {
        Self {
            host_id: json.host_id,
            globally_locked: json.locked,
            buzzer_order: json.buzzer_order,
            players: DashMap::from_iter(json.players.into_iter()),
            scores: json.scores,
            player_join_order: json.player_join_order,
            time_limit: json.time_limit,
        }
    }
}