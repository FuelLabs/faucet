Faucet App
===
[![build](https://github.com/FuelLabs/faucet/actions/workflows/ci.yml/badge.svg)](https://github.com/FuelLabs/faucet/actions/workflows/ci.yml)
[![discord](https://img.shields.io/badge/chat%20on-discord-orange?&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/xfpK4Pe)

A simple faucet app for dispensing tokens on a fuel network. It uses Google captcha for spam resistance
without requiring any social media based identification.

## Configuration
The faucet makes use of environment variables for configuration.

| Environment Variable | Description                                                             |
|----------------------|-------------------------------------------------------------------------|
| RUST_LOG             | EnvFilter configuration for adjusting logging granularity.              |
| HUMAN_LOGGING        | If false, logs will be output as machine readable JSON.                 |
| CAPTCHA_SECRET       | The secret key used for enabling Google captcha authentication.         |
| WALLET_SECRET_KEY    | A hex formatted string of the wallet private key that owns some tokens. |
| FUEL_NODE_URL        | The GraphQL endpoint for connecting to fuel-core.                       |
| SERVICE_PORT         | The port the service will listen for http connections on.               |

## Build and Run

To run locally, assuming environment variables have already been set:

```sh
cargo run
```
