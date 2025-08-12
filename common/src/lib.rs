use std::collections::{HashMap, VecDeque};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    pub players: PlayersMap, // Using im_rc::HashMap on the frontend
    pub scores: HashMap<Uuid, i32>,
    pub player_join_order: Vec<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameStateJson {
    host_id: Uuid,
    locked: bool,
    buzzer_order: VecDeque<(Uuid, String)>,
    players: HashMap<Uuid, Actor>,
    scores: HashMap<Uuid, i32>,
    player_join_order: Vec<Uuid>,
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
        }
    }
}

// Messages from Client to Server
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ClientToServer {
    CreateGame,
    JoinGame {
        game_code: usize,
        player_name: String,
    },
    Buzz {
        game_code: usize,
        player_id: Uuid,
    },
    Lock {
        game_code: usize,
    },
    Unlock {
        game_code: usize,
    },
    Clear {
        game_code: usize,
    },
    UpdateScore {
        game_code: usize,
        player_id: Uuid,
        delta: i32,
    },
}

// Messages from Server to Client
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerToClient {
    GameCreated {
        game_code: usize,
        player_id: Uuid,
        game_state: GameStateJson,
    },
    GameJoined {
        player_id: Uuid,
        player_name: String,
        game_state: GameStateJson,
    },
    GameStateUpdate {
        game_state: GameStateJson,
    },
    Error {
        message: String,
    },
    PlayerBuzzed {
        player_id: Uuid,
        player_name: String,
    },
}
