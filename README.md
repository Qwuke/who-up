# Who's up? A web server for communicating whether there are members in the HackManhattan space

A simple Rust web server for hosting HTTP endpoints and Discord bot for sharing whether non-members can access HackManhattan. Built with [Rocket](https://github.com/sergiobenitez/rocket) and [Twilight](https://github.com/twilight-rs/twilight).

You can add an hour to the time you'll be remaining in the space by making a POST to the /timer endpoint on port 8000, and after incrementing past 6 hours you'll reset the timer back to 0. You can check the current remaining time by making a GET request to the same location. 

Configure the `discord_token` and chosen `discord_channel` id in a file in the root directory called `Secrets.toml`. You can run the project with `cargo run`.