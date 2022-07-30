use colored::Colorize;
use std::vec;

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
            _ => return Err(String::from("")),
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
            usage: usage.to_string(),
            description: description.to_string(),
        }
    }

    pub fn to_string(&self) -> String {
        format!(
            "
    {}
    {}
    {}
    {}
        ",
            "[Usage]".cyan(),
            self.usage,
            "[Description]".cyan(),
            self.description
        )
    }
}

enum Command {
    Rejoin { usernames: Vec<String> },
    Crash { usernames: Vec<String> },
    Ban { usernames: Vec<String> },
    Send,
    SendAll { stage: Stage },
    Unknown,
}

impl Command {
    fn wildcard_filter(list: Vec<String>) -> Vec<String> {
        if list.contains(&String::from("*")) {
            vec!["*".to_string()]
        } else {
            list
        }
    }

    pub fn parse(stdin: String) -> Result<Self, String> {
        let mut splitted: Vec<&str> = stdin.split(' ').collect();

        if splitted.len() == 0 {
            return Ok(Self::Unknown);
        }

        let cmd = splitted.remove(0);

        if splitted.len() == 0 {
            return Err(Self::default_from_str(cmd).help().to_string());
        }

        let parsed = match cmd {
            "rejoin" => Self::Rejoin {
                usernames: Self::wildcard_filter(splitted.iter().map(|s| s.to_string()).collect()),
            },
            "crash" => Self::Crash {
                usernames: Self::wildcard_filter(splitted.iter().map(|s| s.to_string()).collect()),
            },
            "sendall" => Self::SendAll {
                stage: Stage::from_str(splitted.remove(0))?,
            },
            _ => Self::Unknown,
        };

        Ok(parsed)
    }

    pub fn default_from_str(string: &str) -> Self {
        match string {
            "rejoin" => Self::Rejoin { usernames: vec![] },
            "crash" => Self::Crash { usernames: vec![] },
            "ban" => Self::Ban { usernames: vec![] },
            "send" => Self::Send,
            "sendAll" => Self::SendAll { stage: Stage::Cap },
            _ => Self::Unknown,
        }
    }

    pub fn help(&self) -> Help {
        match self {
            Self::Rejoin { usernames: _ } => Help::new(
                "rejoin <username 1|*> <username 2> ...",
                "Will force player to disconnect and reconnect",
            ),
            Self::Crash { usernames: _ } => {
                Help::new("crash <username 1|*> <username 2> ...", "Will crash player")
            }
            Self::Ban { usernames: _ } => {
                Help::new("ban <username 1|*> <username 2> ...", "Will ban player")
            }
            Self::Send => Help::new(
                "send <stage> <id> <scenario[-1..127]> <username 1|*> <username 2> ...",
                "Will teleport player to the wanted place",
            ),
            Self::SendAll { stage: _ } => Help::new(
                "sendall <stage> ",
                "Will teleport players to the wanted stage",
            ),
            Self::Unknown => Help::new("", ""),
        }
    }
}

pub async fn listen() {
    loop {
        for line in std::io::stdin().lines() {
            if line.is_err() {
                continue;
            }

            let _ = Command::parse(line.unwrap());
        }
    }
}
