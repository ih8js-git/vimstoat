pub struct Server {
    pub name: String,
    // TODO: add in channel category separators, also probably add in cache mechanism for channels as well
    pub channels: Vec<Channel>,
}

pub struct Channel {
    pub name: String,
    // will need to be replaced by a view into a cache mechanism
    pub messages: Vec<String>,
    // we have a buffer per channel, such that you can write in a channel
    // and then switch channels such that when you come back, the in-progress draft is still there
    pub buffer: String,
}

// hard coded mock data for development, remove once we've got everything else working
pub static MOCK_SERVERS: std::sync::LazyLock<Vec<Server>> = std::sync::LazyLock::new(|| {
    vec![
        Server {
            name: "Test server".to_string(),
            channels: vec![
                Channel {
                    name: "general".to_string(),
                    messages: vec!["howdy!".to_string()],
                    buffer: String::new(),
                },
                Channel {
                    name: "media".to_string(),
                    messages: vec![
                        "look at this cat!".to_string(),
                        "woahhh so pretty".to_string(),
                        "yessss I love cats!".to_string(),
                    ],
                    buffer: String::new(),
                },
            ],
        },
        Server {
            name: "Epic server".to_string(),
            channels: vec![Channel {
                name: "Now this is epic".to_string(),
                messages: vec!["Epic!!!".to_string()],
                buffer: String::new(),
            }],
        },
    ]
});
