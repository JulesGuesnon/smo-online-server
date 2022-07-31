# SMO online server

> For the non-gamers, SMO stands for Super Mario Odyssey

This server is made for the mod [SMO online](https://github.com/CraftyBoss/SuperMarioOdysseyOnline/), and aims to reproduce the behavior of the [official implementation](https://github.com/Sanae6/SmoOnlineServer). I mainly did it because I like rust, I like SMO, so both togethers are fun.

## Run a server

```bash
docker run --rm -it -v "$PWD/settings.json":/settings.json -p 0.0.0.0:1027:1027 smo
```

```bash
docker run -v "$PWD/settings.json":/settings.json -p 0.0.0.0:1027:1027 -d smo
```
