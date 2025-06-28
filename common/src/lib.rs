use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct Player {
    pub name: String,
}

// Using a special DashMap type that works with Dioxus signals
type PlayersMap = HashMap<Uuid, Player>;


#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct GameState {
    pub host_id: Uuid,
    pub locked: bool,
    pub buzzer_order: Vec<Uuid>,
    pub players: PlayersMap, // Using im_rc::HashMap on the frontend
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameStateJson {
    host_id: Uuid,
    locked: bool,
    buzzer_winner: Vec<Uuid>,
    players: PlayersMap,
}

impl GameState {
    pub fn to_json(&self) -> GameStateJson {
        GameStateJson {
            host_id: self.host_id,
            locked: self.locked,
            buzzer_winner: self.buzzer_order.clone(),
            players: self.players.clone(),
        }
    }
}

impl From<GameStateJson> for GameState {
    fn from(json: GameStateJson) -> Self {
        Self {
            host_id: json.host_id,
            locked: json.locked,
            buzzer_order: json.buzzer_winner,
            players: json.players,
        }
    }
}

// Messages from Client to Server
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum C2S {
    CreateGame { host_name: String },
    JoinGame { game_code: String, player_name: String },
    Buzz { game_code: String, player_id: Uuid },
    Lock { game_code: String },
    Unlock { game_code: String },
    Clear { game_code: String },
}

// Messages from Server to Client
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum S2C {
    GameCreated { game_code: String, player_id: Uuid, game_state: GameStateJson },
    GameJoined { player_id: Uuid, game_state: GameStateJson },
    GameStateUpdate { game_state: GameStateJson },
    Error { message: String },
}
