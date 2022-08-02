# SMO online server

> For the non-gamers, SMO stands for Super Mario Odyssey

This server is made for the mod [SMO online](https://github.com/CraftyBoss/SuperMarioOdysseyOnline/), and aims to reproduce the behavior of the [official implementation](https://github.com/Sanae6/SmoOnlineServer). I mainly did it because I like rust, I like SMO, so both togethers are fun.

## What's the difference with the official server ?

### Features

For now, the main difference is that this server doesn't implement a Discord bot, and I don't know if it will.

### Performances

:warning: I made a test with a modded Switch and Yuzu to have 2 players moving at the same time, both servers were running on my computer: a Mac M1 Pro, and on local network. **So the following numbers are here to give an idea but aren't representative of all cases.**

| Server   | RAM (average) | Cpu % (average) |
| -------- | ------------- | --------------- |
| Official | 47mo          | 9%              |
| This one | 3.7mo         | 0.7%            |

## Run a server

There's mutliple ways to run a server

### Download an exe

Go to the [releases](https://github.com/JulesGuesnon/smo-online-server/releases) and download the file that corresponds to your OS and extract the executable for the archive.
For MacOS you'll probably have an integrity error if you try to open it. To make it works, you have to: right click -> open -> open.

### Run with docker

If you just want to run a temporary container you can copy/paste the following command:

```bash
docker run --rm -it -v "$PWD/settings.json":/settings.json -p 0.0.0.0:1027:1027 ghcr.io/julesguesnon/smo-online-server
```

If you want to run a container in the background run the following command. Note the **`-it`** flag that is **required** to make it works

```bash
docker run -d -it -v "$PWD/settings.json":/settings.json -p 0.0.0.0:1027:1027 ghcr.io/julesguesnon/smo-online-server
```

If you want to run commands, you'll need to attach the container to your terminal. To do so run:

1. `docker ps` and copy the id of the container
2. `docker attach <id>` and voilà (replace `<id>` not only `id`)! Type `help` or `press enter` to show the help.
3. :warning: If you `ctrl+c` to exit the console, **you'll stop the server**. To exit the console without stopping the server, do: `ctrl+pq`

## Server commands

When the server is launched you can type `help` or `press enter` to get a list of commands that you can use to manage the server.

### How to allow everyone to connect to the server?

If you don't know how to make everyone connect to your server, follow [this guide](./docs/connect.md), there's also instructions to deploy to [fly.io](https://fly.io)

## Credits

Thanks a lot to:

- [CraftyBoss](https://github.com/CraftyBoss) for the mod
- [Sanae6](https://github.com/Sanae6) for the official server
