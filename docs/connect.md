# How to allow everyone to connect to the server?

When you'll run the server on your computer, only people on the same network will be able to connect to the server. To allow everyone to play, we have to find a way to expose the server to the whole internet.
I won't make a tutorial for everything, I'll only put some keywords that you can google (or search on youtube) to find your answer.

## Open a port on your box

You may see people that advice you to do that, **please don't**. You may expose yourself and your entire local network to security issues, so please, only do it if you know what you're doing.

Keywords:

- port forwarding box
- open port box

## Setup a VPN

A VPN will make as if all the computers are connected to the same network (just like Hamachi when we wanted to play Minecraft). So this option only works if **everyone plays on an emulator**.

Keywords:

- setup vps
- create vps
- setup openvpn on vps
- install openvpn on vps

## Use a cloud provider

This is probably the best way to deploy the server, and moreover for free. There's a lot of providers that allow to deploy applications, and they often have free tiers.

### Deploy with Fly.io

One of them is [fly.io](https://fly.io), a provider that allows to deploy applications based on location and that has a generous free tier.
**It requires a credit card** to use the free tier. You won't pay anything as long as you stay in free tier quotas (which is really really likely), they only need it to avoid bot from abusing their free tier.

**Note** that, this approach will require you to use the terminal, here is [a tutorial](https://medium.com/@grace.m.nolan/terminal-for-beginners-e492ba10902a) to get familliar with it (even if you are on Windows, most of the commands will work in Powershell).

#### Instructions

1. Install [Flyctl](https://fly.io/docs/getting-started/installing-flyctl/)
2. [Login](https://fly.io/docs/getting-started/log-in-to-fly/)
3. Create a folder

```
mkdir smo-online-server
```

4. Go into the folder

```
cd smo-online-server
```

5. Create the configuration file by running `flyctl launch --image ghcr.io/julesguesnon/smo-online-server`

   - Choose the name you want
   - Select the region that is the closest from you
   - Setup a Postgresql: No
   - Deploy now: No

6. Open `fly.toml` file in a text editor, and makes it look like that

```toml
# fly.toml file generated for smo-online-server on 2022-08-01T13:22:37+02:00

app = "<app-name>"
kill_signal = "SIGINT"
kill_timeout = 5
processes = []

[build]
  image = "ghcr.io/julesguesnon/smo-online-server"

[env]

[experimental]
  allowed_public_ports = []
  auto_rollback = true

[[services]]
  http_checks = []
  internal_port = 1027
  processes = ["app"]
  protocol = "tcp"
  script_checks = []
  [services.concurrency]
    hard_limit = 25
    soft_limit = 20
    type = "connections"

  [[services.ports]]
    handlers = []
    port = 1027

```

7. Deploy the app by running `flyctl deploy`

8. Now go to your [apps](https://fly.io/apps/)

9. Click on the app you just deployed

10. In `Application information` you should see the ip, this is what you want to put in the mod!

Congratulations, the server is deployed!
