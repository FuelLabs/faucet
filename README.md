# Faucet App

[![build](https://github.com/FuelLabs/faucet/actions/workflows/ci.yml/badge.svg)](https://github.com/FuelLabs/faucet/actions/workflows/ci.yml)
[![discord](https://img.shields.io/badge/chat%20on-discord-orange?&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/xfpK4Pe)

A simple faucet app for dispensing tokens on a fuel network. It uses Google captcha for spam resistance
without requiring any social media based identification.

## Configuration

The faucet makes use of environment variables for configuration.

| Environment Variable | Description                                                                                     |
| -------------------- | ----------------------------------------------------------------------------------------------- |
| RUST_LOG             | EnvFilter configuration for adjusting logging granularity.                                      |
| HUMAN_LOGGING        | If false, logs will be output as machine readable JSON.                                         |
| CAPTCHA_SECRET       | The secret key used for enabling Google captcha authentication.                                 |
| CAPTCHA_KEY          | The website key used for enabling Google captcha authentication.                                |
| WALLET_SECRET_KEY    | A hex formatted string of the wallet private key that owns some tokens.                         |
| FUEL_NODE_URL        | The GraphQL endpoint for connecting to fuel-core.                                               |
| PUBLIC_FUEL_NODE_URL | The public GraphQL endpoint for connecting to fuel-core. Ex.: https://node.fuel.network/graphql |
| SERVICE_PORT         | The port the service will listen for http connections on.                                       |
| DISPENSE_AMOUNT      | Dispense amount on each faucet                                                                  |
| MIN_GAS_PRICE        | The minimum gas price to use in each transfer                                                   |

## Build and Run

To run locally, assuming environment variables have already been set:

```sh
cargo run
```

## Query Params

When integrating the faucet you can use the following query params:

| Name     | Type    | Description                                                                                |
| -------- | ------- | ------------------------------------------------------------------------------------------ |
| address  | Address | Auto-fill the address field using the provided value                                       |
| redirect | Boolean | If true once the faucet is complete, the user will be redirected back to the referrer page |

Ex.: `http://localhost:3000/?address=fuel134ddh9pfsspar086htdldwxq2jsr3yvqtj5w456kkrz3s653la5q347kmc&redirect=true`
