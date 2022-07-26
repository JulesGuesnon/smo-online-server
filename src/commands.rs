use std::fmt::Display;
use std::process::exit;
use std::str::FromStr;
use std::string::ToString;
use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use owo_colors::OwoColorize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::sleep;
use tracing::{error, info};
use uuid::Uuid;

use crate::packet::{Content, Packet, TagUpdate};
use crate::server::Server;
use crate::settings::{FlipPov, Settings};

trait IsWildcard {
    fn is_wildcard(&self) -> bool;
}

impl IsWildcard for Vec<String> {
    fn is_wildcard(&self) -> bool {
        self.contains(&String::from("*"))
    }
}

#[derive(Debug)]
pub enum Stage {
    Cap,
    Cascade,
    Sand,
    Lake,
    Wooded,
    Cloud,
    Lost,
    Metro,
    Sea,
    Snow,
    Lunch,
    Ruined,
    Bowser,
    Moon,
    Mush,
    Dark,
    Darker,
}

impl Stage {
    pub fn help() -> String {
        format!(
            "
Here is the list of the valid stages

{}
- Cap
- Cascade
- Sand
- Lake
- Wooded
- Cloud
- Lost
- Metro
- Sea
- Snow
- Lunch
- Ruined
- Bowser
- Moon
- Mush
- Dark
- Darker
        ",
            "[Stages]".cyan()
        )
    }

    pub fn from_str(string: &str) -> Result<Self, String> {
        let stage = match string.to_lowercase().as_str() {
            "cap" => Self::Cap,
            "cascade" => Self::Cascade,
            "sand" => Self::Sand,
            "lake" => Self::Lake,
            "wooded" => Self::Wooded,
            "cloud" => Self::Cloud,
            "lost" => Self::Lost,
            "metro" => Self::Metro,
            "sea" => Self::Sea,
            "snow" => Self::Snow,
            "lunch" => Self::Lunch,
            "ruined" => Self::Ruined,
            "bowser" => Self::Bowser,
            "moon" => Self::Moon,
            "mush" => Self::Mush,
            "dark" => Self::Dark,
            "darker" => Self::Darker,
            _ => return Err(Self::help()),
        };

        Ok(stage)
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Cap => "CapWorldHomeStage",
            Self::Cascade => "WaterfallWorldHomeStage",
            Self::Sand => "SandWorldHomeStage",
            Self::Lake => "LakeWorldHomeStage",
            Self::Wooded => "ForestWorldHomeStage",
            Self::Cloud => "CloudWorldHomeStage",
            Self::Lost => "ClashWorldHomeStage",
            Self::Metro => "CityWorldHomeStage",
            Self::Sea => "SeaWorldHomeStage",
            Self::Snow => "SnowWorldHomeStage",
            Self::Lunch => "LavaWorldHomeStage",
            Self::Ruined => "BossRaidWorldHomeStage",
            Self::Bowser => "SkyWorldHomeStage",
            Self::Moon => "MoonWorldHomeStage",
            Self::Mush => "PeachWorldHomeStage",
            Self::Dark => "Special1WorldHomeStage",
            Self::Darker => "Special2WorldHomeStag",
        }
    }
}
pub struct Help {
    pub usage: String,
    pub description: String,
}

impl Help {
    pub fn new(usage: &str, description: &str) -> Self {
        Self {
            usage: usage.to_owned(),
            description: description.to_owned(),
        }
    }

    pub fn merge(helps: Vec<Help>) -> Self {
        helps.into_iter().fold(
            Self {
                usage: "".to_owned(),
                description: "Enter one of the command above to get informations about it"
                    .to_owned(),
            },
            |mut acc, help| {
                acc.usage = format!(
                    "{}{}{}",
                    acc.usage,
                    if acc.usage.is_empty() { "" } else { "\n" },
                    help.usage
                );
                acc
            },
        )
    }
}

impl Display for Help {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.description.is_empty() {
            write!(f, "{}\n{}\n", "[Usage]".cyan(), self.usage,)
        } else {
            write!(
                f,
                "{}\n{}\n\n{}\n{}\n",
                "[Usage]".cyan(),
                self.usage,
                "[Description]".cyan(),
                self.description
            )
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum TagState {
    Seeker,
    Hider,
}

#[derive(Debug)]
pub enum TagSubCmd {
    Time {
        username: String,
        minutes: u16,
        seconds: u8,
    },
    Seeking {
        username: String,
        state: TagState,
    },
    Start {
        time: u8,
        seekers: Vec<String>,
    },
}

#[derive(Debug)]
pub enum FlipSubCmd {
    List,
    Add { user_id: Uuid },
    Remove { user_id: Uuid },
    Set { enabled: bool },
    Pov { pov: FlipPov },
}

#[derive(Debug)]
pub enum ShineSubCmd {
    List,
    Clear,
    Sync,
    Send { id: i32, players: Vec<String> },
}

#[derive(Debug)]
pub enum Command {
    Rejoin {
        players: Vec<String>,
    },
    Crash {
        players: Vec<String>,
    },
    Ban {
        players: Vec<String>,
    },
    Send {
        stage: Stage,
        id: String,
        scenario: i8,
        players: Vec<String>,
    },
    SendAll {
        stage: Stage,
    },
    Scenario {
        subcmd: String,
        value: String,
    },
    MaxPlayers {
        count: u16,
    },
    List,
    LoadSettings,
    Tag {
        subcmd: TagSubCmd,
    },
    Flip {
        subcmd: FlipSubCmd,
    },
    Shine {
        subcmd: ShineSubCmd,
    },
    Stop,
    Unknown {
        cmd: String,
    },
}

impl Command {
    fn wildcard_filter(list: Vec<String>) -> Vec<String> {
        if list.contains(&String::from("*")) {
            vec!["*".to_owned()]
        } else {
            list
        }
    }

    pub fn parse(stdin: String) -> Result<Self, String> {
        let mut splitted: Vec<&str> = stdin.split(' ').filter(|v| !(*v).is_empty()).collect();

        if splitted.is_empty() {
            return Ok(Self::Unknown { cmd: "".to_owned() });
        }

        let cmd = splitted.remove(0);

        if splitted.is_empty() && (cmd != "list" && cmd != "stop" && cmd != "loadsettings") {
            let cmd = Self::default_from_str(cmd);
            return match &cmd {
                Self::Unknown { cmd: _ } => Ok(cmd),
                _ => Err(cmd.help().to_string()),
            };
        }

        let parsed = match cmd {
            "rejoin" => Self::Rejoin {
                players: Self::wildcard_filter(splitted.iter().map(|s| s.to_lowercase()).collect()),
            },
            "crash" => Self::Crash {
                players: Self::wildcard_filter(splitted.iter().map(|s| s.to_lowercase()).collect()),
            },
            "ban" => Self::Ban {
                players: Self::wildcard_filter(splitted.iter().map(|s| s.to_lowercase()).collect()),
            },
            "sendall" => Self::SendAll {
                stage: Stage::from_str(splitted.remove(0))?,
            },
            "send" if splitted.len() < 4 => {
                return Err(Self::default_from_str("send").help().to_string());
            }
            "send" => Self::Send {
                stage: Stage::from_str(splitted.remove(0))?,
                id: splitted.remove(0).to_owned(),
                scenario: splitted
                    .remove(0)
                    .parse::<i8>()
                    .map_err(|_| "Scenario should be a number between -1 and 127".to_owned())?,
                players: Self::wildcard_filter(splitted.iter().map(ToString::to_string).collect()),
            },
            "scenario" if splitted.len() < 2 => {
                return Err(Self::default_from_str("scenario").help().to_string());
            }
            "scenario" => Self::Scenario {
                subcmd: splitted.remove(0).to_owned(),
                value: splitted.remove(0).to_owned(),
            },
            "maxplayers" if splitted.is_empty() => {
                return Err(Self::default_from_str("maxplayers").help().to_string());
            }
            "maxplayers" => Self::MaxPlayers {
                count: splitted
                    .remove(0)
                    .parse::<u16>()
                    .map_err(|_| "Count should be a positive integer")?,
            },
            "list" => Self::List,
            "tag" if splitted.len() < 4 => {
                return Err(Self::default_from_str("tag").help().to_string());
            }
            "tag" => {
                let subcmd = splitted.remove(0);

                match subcmd {
                    "time" if splitted.len() == 3 => Self::Tag {
                        subcmd: TagSubCmd::Time {
                            username: splitted.remove(0).to_owned(),
                            minutes: splitted.remove(0).parse().map_err(|_| {
                                "Invalid mintues, value should be an integer between 0 and 65535"
                            })?,
                            seconds: splitted.remove(0).parse().map_err(|_| {
                                "Invalid seconds, value should be an integer between 0 and 255"
                            })?,
                        },
                    },
                    "seeking" if splitted.len() == 2 => Self::Tag {
                        subcmd: TagSubCmd::Seeking {
                            username: splitted.remove(0).to_owned(),
                            state: match splitted.remove(0) {
                                "seeker" => TagState::Seeker,
                                "hider" => TagState::Hider,
                                v => {
                                    return Err(format!(
                                        "Invalid value '{}', expected 'seeker' or 'hider'",
                                        v
                                    ));
                                }
                            },
                        },
                    },
                    "start" if splitted.len() >= 2 => Self::Tag {
                        subcmd: TagSubCmd::Start {
                            time: splitted
                                .remove(0)
                                .parse()
                                .map_err(|_| "Invalid time, value should be between 0 and 255")?,
                            seekers: splitted.into_iter().map(String::from).collect(),
                        },
                    },
                    _ => {
                        return Err(Self::default_from_str("tag").help().to_string());
                    }
                }
            }
            "flip" if splitted.is_empty() => {
                return Err(Self::default_from_str("flip").help().to_string());
            }
            "flip" => match splitted.remove(0) {
                "list" => Command::Flip {
                    subcmd: FlipSubCmd::List,
                },
                "add" if splitted.len() == 1 => Command::Flip {
                    subcmd: FlipSubCmd::Add {
                        user_id: Uuid::from_str(splitted.remove(0))
                            .map_err(|_| "Invalid player id")?,
                    },
                },
                "remove" if splitted.len() == 1 => Command::Flip {
                    subcmd: FlipSubCmd::Remove {
                        user_id: Uuid::from_str(splitted.remove(0))
                            .map_err(|_| "Invalid player id")?,
                    },
                },
                "set" if splitted.len() == 1 => Command::Flip {
                    subcmd: FlipSubCmd::Set {
                        enabled: splitted
                            .remove(0)
                            .parse()
                            .map_err(|_| "Invalid value, expected true or false")?,
                    },
                },
                "pov" if splitted.len() == 1 => Command::Flip {
                    subcmd: FlipSubCmd::Pov {
                        pov: FlipPov::from_str(splitted.remove(0))?,
                    },
                },
                _ => {
                    return Err(Self::default_from_str("flip").help().to_string());
                }
            },
            "shine" => match splitted.remove(0) {
                "list" => Self::Shine {
                    subcmd: ShineSubCmd::List,
                },
                "clear" => Self::Shine {
                    subcmd: ShineSubCmd::Clear,
                },
                "sync" => Self::Shine {
                    subcmd: ShineSubCmd::Sync,
                },
                "send" if splitted.len() >= 2 => Self::Shine {
                    subcmd: ShineSubCmd::Send {
                        id: splitted
                            .remove(0)
                            .parse()
                            .map_err(|_| "Invalid moon id, it should be a number")?,
                        players: Self::wildcard_filter(
                            splitted.into_iter().map(String::from).collect(),
                        ),
                    },
                },
                _ => return Err(Self::default_from_str("shine").help().to_string()),
            },
            "stop" => Self::Stop,
            "loadsettings" => Self::LoadSettings,
            v => Self::Unknown { cmd: v.to_owned() },
        };

        Ok(parsed)
    }

    pub fn default_from_str(string: &str) -> Self {
        match string {
            "rejoin" => Self::Rejoin { players: vec![] },
            "crash" => Self::Crash { players: vec![] },
            "ban" => Self::Ban { players: vec![] },
            "send" => Self::Send {
                stage: Stage::Cap,
                id: "".to_owned(),
                scenario: 0,
                players: vec![],
            },
            "sendall" => Self::SendAll { stage: Stage::Cap },
            "scenario" => Self::Scenario {
                subcmd: "".to_owned(),
                value: "".to_owned(),
            },
            "maxplayers" => Self::MaxPlayers { count: 0 },
            "list" => Self::List,
            "loadsettings" => Self::LoadSettings,
            "tag" => Self::Tag {
                subcmd: TagSubCmd::Seeking {
                    username: "".to_owned(),
                    state: TagState::Hider,
                },
            },
            "flip" => Self::Flip {
                subcmd: FlipSubCmd::List,
            },
            "shine" => Self::Shine {
                subcmd: ShineSubCmd::List,
            },
            "stop" => Self::Stop,
            v => Self::Unknown { cmd: v.to_owned() },
        }
    }

    pub fn help(&self) -> Help {
        match self {
            Self::Rejoin { players: _ } => Help::new(
                "rejoin <username 1|*> <username 2> ...",
                "Will force player to disconnect and reconnect",
            ),
            Self::Crash { players: _ } => {
                Help::new("crash <username 1|*> <username 2> ...", "Will crash player")
            }
            Self::Ban { players: _ } => {
                Help::new("ban <username 1|*> <username 2> ...", "Will ban player")
            }
            Self::Send {
                stage: _,
                id: _,
                scenario: _,
                players: _,
            } => Help::new(
                "send <stage> <id> <scenario[-1..127]> <username 1|*> <username 2> ...",
                "Will teleport player to the wanted stage and scenario",
            ),
            Self::SendAll { stage: _ } => Help::new(
                "sendall <stage> ",
                "Will teleport players to the wanted stage",
            ),
            Self::Scenario {
                subcmd: _,
                value: _,
            } => Help::new("scenario merge <true|false>", "Will merge scenarios"),
            Self::MaxPlayers { count: _ } => Help::new(
                "maxplayers <count>",
                "Will update the max player that can connect to the server",
            ),
            Self::List => Help::new("list", "List all the connected players"),
            Self::LoadSettings => Help::new("loadsettings", "Load the settings into the server. Do ift after changing the settings while the server is running"),
            Self::Tag { subcmd: _ } => {
                let time_usage = "tag time <username|*> <mintues[0-65535]> <seconds[0-59]>";
                let time_desc = format!("- {} set the time for 1 player or everyone if username is *", "tag time".cyan());

                let seeking = "tag seeking <username|*> <hider|seeker>";
                let seeking_desc = format!("- {} allows to set the player as a hider or seeker. You can set everyone role if the username is *", "tag seeking".cyan());

                let start = "tag start <time[0-255]> <username 1> <username 2> ...";
                let start_desc = format!("- {} will start the game after the input time is over and set the input players to seeker and the rest to hider", "tag start".cyan());

                Help::new(
                    &format!("{}\n{}\n{}", time_usage, seeking, start),
                    &format!("{}\n{}\n{}", time_desc, seeking_desc, start_desc)
                )
            },
            Self::Flip { subcmd: _ } => {
                let list = "flip list";
                let list_desc = format!("- {} list the ids of the flipped players", "flip list".cyan());

                let add = "flip add <user id>";
                let add_desc = format!("- {} will add a user to the flip list", "flip add".cyan());

                let remove = "flip remove <user id>";
                let remove_desc = format!("- {} will remove a user to the flip list", "flip remove".cyan());

                let set = "flip set <true|false>";
                let set_desc = format!("- {} will enable or disable flip", "flip set".cyan());

                let pov = "flip pov <self|others|both>";
                let pov_desc = format!("- {} will update the point of view", "flip pov".cyan());


                Help::new(
                    &format!("{}\n{}\n{}\n{}\n{}", list, add, remove, set, pov),
                    &format!("{}\n{}\n{}\n{}\n{}", list_desc, add_desc, remove_desc, set_desc, pov_desc)
                )
            },
            Self::Shine { subcmd: _ } => {
                let list = "shine list";
                let list_desc = format!("- {} list the ids of the collected moons", "shine list".cyan());

                let clear = "shine clear";
                let clear_desc = format!("- {} will delete all the collected moons", "shine clean".cyan());

                let sync = "shine sync";
                let sync_desc = format!("- {} will force the sync of the moons", "shine sync".cyan());

                let send = "shine send <id> <username 1|*> <username 2> ...";
                let send_desc = format!("- {} will send a moon to a player or everyone if username is *", "shine send".cyan());


                Help::new(
                    &format!("{}\n{}\n{}\n{}", list, clear, sync, send),
                    &format!("{}\n{}\n{}\n{}", list_desc, clear_desc, sync_desc, send_desc)
                )
            },
            Self::Stop => Help::new("stop", "Will stop the server"),
            Self::Unknown { cmd: _ } => Help::merge(vec![
                Self::default_from_str("rejoin").help(),
                Self::default_from_str("crash").help(),
                Self::default_from_str("ban").help(),
                Self::default_from_str("send").help(),
                Self::default_from_str("sendall").help(),
                Self::default_from_str("scenario").help(),
                Self::default_from_str("maxplayers").help(),
                Self::default_from_str("list").help(),
                Self::default_from_str("loadsettings").help(),
                Self::default_from_str("tag").help(),
                Self::default_from_str("flip").help(),
                Self::default_from_str("shine").help(),
                Self::default_from_str("stop").help(),
            ]),
        }
    }
}

pub async fn listen(server: Arc<Server>) {
    let mut stdin = BufReader::new(tokio::io::stdin()).lines();

    let task = async move {
        loop {
            let line = stdin.next_line().await;

            if line.is_err() {
                error!("Failed to read stdin {}", line.unwrap_err());
                continue;
            }

            let line = line.unwrap();

            if let Some(line) = line {
                match Command::parse(line) {
                    Ok(cmd) => exec_cmd(server.clone(), cmd).await,
                    Err(message) => println!("\n{}\n{}", "[Error]".red(), message),
                };
            }
        }
    };

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Stopping the server");
            exit(0);
        },
        _ = task => {}
    };
}

async fn exec_cmd(server: Arc<Server>, cmd: Command) {
    match cmd {
        Command::Rejoin { players } if players.is_wildcard() => {
            server.disconnect_all().await;
            info!("Disconnected everyone");
        }
        Command::Rejoin { players } => {
            server.disconnect_by_name(players.clone()).await;
            info!("Disconnected {}", players.join(", "));
        }
        Command::Crash { players } if players.is_wildcard() => {
            server
                .broadcast(Packet::new(
                    Uuid::nil(),
                    Content::ChangeStage {
                        stage: "baguette".to_owned(),
                        id: "dufromage".to_owned(),
                        scenario: 21,
                        sub_scenario: 42,
                    },
                ))
                .await;

            info!("Crashed everyone");
        }
        Command::Crash { players } => {
            server
                .broadcast_map(
                    Packet::new(
                        Uuid::nil(),
                        Content::ChangeStage {
                            stage: "baguette".to_owned(),
                            id: "dufromage".to_owned(),
                            scenario: 21,
                            sub_scenario: 42,
                        },
                    ),
                    |player, packet| {
                        let players = players.clone();
                        async move {
                            let player = player.read().await;

                            if players.contains(&player.name) {
                                Some(packet)
                            } else {
                                None
                            }
                        }
                    },
                )
                .await;

            info!("Crashed {}", players.join(", "));
        }
        Command::Send {
            stage,
            id,
            scenario,
            players,
        } if players.is_wildcard() => {
            server
                .broadcast(Packet::new(
                    Uuid::nil(),
                    Content::ChangeStage {
                        id: id.clone(),
                        stage: stage.to_str().to_owned(),
                        scenario,
                        sub_scenario: 0,
                    },
                ))
                .await;

            info!(
                "Sent everyone to stage: {}, id: {}, scenario: {}",
                stage.to_str(),
                id,
                scenario
            );
        }
        Command::Send {
            stage,
            id,
            scenario,
            players,
        } => {
            server
                .broadcast_map(
                    Packet::new(
                        Uuid::nil(),
                        Content::ChangeStage {
                            id: id.clone(),
                            stage: stage.to_str().to_owned(),
                            scenario,
                            sub_scenario: 0,
                        },
                    ),
                    |player, packet| {
                        let players = players.clone();
                        async move {
                            let player = player.read().await;

                            if players.contains(&player.name) {
                                Some(packet)
                            } else {
                                None
                            }
                        }
                    },
                )
                .await;

            info!(
                "Sent everyone to stage: {}, id: {}, scenario: {}",
                stage.to_str(),
                id,
                scenario
            );
        }
        Command::SendAll { stage } => {
            server
                .broadcast(Packet::new(
                    Uuid::nil(),
                    Content::ChangeStage {
                        id: "".to_owned(),
                        stage: stage.to_str().to_owned(),
                        scenario: -1,
                        sub_scenario: 0,
                    },
                ))
                .await;

            info!("Sent everyone to {}", stage.to_str());
        }
        Command::Ban { players } => {
            let mut settings = server.settings.write().await;
            let peers = server.peers.read().await;

            for name in players.clone() {
                let id = server.players.get_id_by_name(name).await;

                if id.is_none() {
                    continue;
                }

                let id = id.unwrap();

                let peer = peers.get(&id);

                if peer.is_none() {
                    settings.ban_list.ban(id, None);
                    settings.save().await;
                    break;
                }

                let peer = peer.unwrap();
                settings.ban_list.ban(id, Some(peer.ip));

                peer.send(Packet::new(
                    Uuid::nil(),
                    Content::ChangeStage {
                        stage: "baguette".to_owned(),
                        id: "dufromage".to_owned(),
                        scenario: 21,
                        sub_scenario: 42,
                    },
                ))
                .await;
                settings.save().await;
            }

            info!("Banned {}", players.join(", "));
        }
        Command::Scenario { subcmd, value } => match subcmd.as_str() {
            "merge" => {
                let mut settings = server.settings.write().await;
                if value.as_str() == "true" {
                    settings.scenario.merge_enabled = true;
                    settings.save().await;
                    info!("Updated merge to {}", true);
                } else if value.as_str() == "false" {
                    settings.scenario.merge_enabled = false;
                    settings.save().await;
                    info!("Updated merge to {}", false);
                } else {
                    println!("{}", Command::default_from_str("scenario").help())
                }
            }
            _ => println!("{}", Command::default_from_str("scenario").help()),
        },
        Command::MaxPlayers { count } => {
            let mut settings = server.settings.write().await;

            settings.server.max_players = count as i16;
            settings.save().await;

            info!("Updated max players to {}", count);
        }
        Command::List => {
            let connected = server.connected_peers().await;

            let players = server.players.all_from_ids(connected).await;

            let players = join_all(players.iter().map(|p| p.read())).await;

            let list = players.iter().fold(String::from(""), |acc, player| {
                format!(
                    "{}{}- [{}] -> {}",
                    acc,
                    if acc.is_empty() { "" } else { "\n" },
                    player.name,
                    player.id
                )
            });

            println!("Connected players: \n{}", list);
        }
        Command::LoadSettings => {
            let updated = Settings::load().await;

            let mut settings = server.settings.write().await;

            *settings = updated;
        }
        Command::Tag {
            subcmd:
                TagSubCmd::Time {
                    username,
                    minutes,
                    seconds,
                },
        } => {
            let packet = Packet::new(
                Uuid::nil(),
                Content::Tag {
                    update_type: TagUpdate::Time.as_byte(),
                    is_it: false,
                    seconds: u16::from(seconds),
                    minutes,
                },
            );

            if username.as_str() == "*" {
                server.broadcast(packet).await;
            } else if let Some(id) = server.players.get_id_by_name(username.clone()).await {
                match server.send_to(&id, packet).await {
                    Ok(_) => info!("Updated time of {}", username),
                    Err(_) => info!("Couldn't find player {}", username),
                }
            }
        }
        Command::Tag {
            subcmd: TagSubCmd::Seeking { username, state },
        } => {
            let packet = Packet::new(
                Uuid::nil(),
                Content::Tag {
                    update_type: TagUpdate::State.as_byte(),
                    is_it: state == TagState::Seeker,
                    seconds: 0,
                    minutes: 0,
                },
            );

            if username.as_str() == "*" {
                server.broadcast(packet).await;
            } else if let Some(id) = server.players.get_id_by_name(username.clone()).await {
                match server.send_to(&id, packet).await {
                    Ok(_) => info!("Updated time of {}", username),
                    Err(_) => info!("Couldn't find player {}", username),
                }
            }
        }
        Command::Tag {
            subcmd:
                TagSubCmd::Start {
                    time,
                    seekers: will_seek,
                },
        } => {
            tokio::spawn(async move {
                sleep(Duration::from_secs(u64::from(time))).await;

                let players = server.players.all_ids_and_names().await;

                let [seekers, hiders] = players.into_iter().fold(
                    [vec![], vec![]],
                    |[mut seekers, mut hiders], (id, username)| {
                        if will_seek.contains(&username) {
                            seekers.push(id);
                        } else {
                            hiders.push(id);
                        }

                        [seekers, hiders]
                    },
                );

                let peers = server.peers.read().await;

                for id in seekers {
                    if let Some(peer) = peers.get(&id) {
                        peer.send(Packet::new(
                            Uuid::nil(),
                            Content::Tag {
                                update_type: TagUpdate::State.as_byte(),
                                is_it: true,
                                seconds: 0,
                                minutes: 0,
                            },
                        ))
                        .await
                    }
                }

                for id in hiders {
                    if let Some(peer) = peers.get(&id) {
                        peer.send(Packet::new(
                            Uuid::nil(),
                            Content::Tag {
                                update_type: TagUpdate::State.as_byte(),
                                is_it: false,
                                seconds: 0,
                                minutes: 0,
                            },
                        ))
                        .await
                    }
                }
            });
        }
        Command::Flip {
            subcmd: FlipSubCmd::List,
        } => {
            let settings = server.settings.read().await;

            info!(
                "User ids: {}",
                settings
                    .flip
                    .players
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<String>>()
                    .join(", ")
            );
        }
        Command::Flip {
            subcmd: FlipSubCmd::Add { user_id },
        } => {
            let settings = server.settings.read().await;

            if !settings.flip.players.contains(&user_id) {
                drop(settings);
                let mut settings = server.settings.write().await;
                settings.flip.players.push(user_id);

                settings.save().await;

                info!("Added {} to flip list", user_id);
            } else {
                info!("Player {} was already in the list", user_id);
            }
        }
        Command::Flip {
            subcmd: FlipSubCmd::Remove { user_id },
        } => {
            let settings = server.settings.read().await;

            if settings.flip.players.contains(&user_id) {
                drop(settings);
                let mut settings = server.settings.write().await;
                settings.flip.players.retain(|v| *v != user_id);

                settings.save().await;

                info!("Removed {} from the flip list", user_id);
            } else {
                info!("Player {} wasn't in the list", user_id);
            }
        }
        Command::Flip {
            subcmd: FlipSubCmd::Set { enabled },
        } => {
            let mut settings = server.settings.write().await;
            settings.flip.enabled = enabled;

            settings.save().await;

            info!("{} flip", if enabled { "Enabled" } else { "Disabled" });
        }
        Command::Flip {
            subcmd: FlipSubCmd::Pov { pov },
        } => {
            let mut settings = server.settings.write().await;
            settings.flip.pov = pov.clone();

            settings.save().await;

            info!("Set pov to {}", pov.to_str());
        }
        Command::Shine {
            subcmd: ShineSubCmd::List,
        } => {
            let bag = server.shine_bag.read().await;

            let string = bag
                .iter()
                .fold("".to_owned(), |acc, id| format!("{}{}{}", acc, id, ", "));

            info!("{}", string);
        }
        Command::Shine {
            subcmd: ShineSubCmd::Clear,
        } => {
            let mut bag = server.shine_bag.write().await;

            bag.clear();

            info!("Cleared all the moons");
        }
        Command::Shine {
            subcmd: ShineSubCmd::Sync,
        } => {
            server.sync_shine_bag().await;

            info!("Synced moons");
        }
        Command::Shine {
            subcmd: ShineSubCmd::Send { id, players },
        } => {
            let packet = Packet::new(Uuid::nil(), Content::Shine { id });

            if players.is_wildcard() {
                server.broadcast(packet).await
            } else {
                let peers = server.peers.read().await;

                for username in players.clone() {
                    let id = server.players.get_id_by_name(username).await;

                    if id.is_none() {
                        continue;
                    }

                    let id = id.unwrap();

                    if let Some(peer) = peers.get(&id) {
                        peer.send(packet.clone()).await;
                    }
                }
            }

            info!("Sent moon {} to {}", id, players.join(", "));
        }
        Command::Stop => {
            exit(0);
        }
        Command::Unknown { cmd } => {
            println!(
                "\n{} {}\n\n{}",
                "Invalid command:".red(),
                cmd,
                Command::Unknown { cmd: "".to_owned() }.help()
            );
        }
    }
}
