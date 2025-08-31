use serde::{Deserialize, Serialize};
use uuid::Uuid;
use super::*;

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
    // StartCountDown {
    //     countdown: usize
    // },
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
    // StartCountDown {
    //     countdown: usize
    // },
}
