# FactorishWasm

A port of [FactorishJS](https://github.com/msakuta/FactorishJS) to Wasm/Rust (and a bit of HTML5+JavaScript).

Try it now!
https://msakuta.github.io/FactorishWasm/index.html


## Features

This project is a demonstration that how HTML5 and [WebAssembly](https://developer.mozilla.org/en-US/docs/WebAssembly/Concepts)
(written in [Rust](https://www.rust-lang.org/)) can be used to create a game
with complexity like the great game [Factorio](https://store.steampowered.com/app/427520/Factorio/).

Mozilla Development Network itself has a [WebAssembly tutorial](https://developer.mozilla.org/en-US/docs/WebAssembly/Rust_to_wasm) in Rust and wasm-pack,
which is extremely easy to get started.

## Prerequisites

This game uses JavaScript and WebAssembly (Wasm), so you need a browser with WebAssembly support.
Most modern browser support it nowadays.



## How to build and run

Install

* Cargo >1.40
* npm >7.0.2

Install npm packages

    npm i

### Launch development server

    npm start

It will start webpack-dev-server, launch a browser and show http://localhost:8080 automatically.

### Launch production distribution

    npm run build

### Building WebAssembly module manually

Usually you don't have to run the commands in this section.
Just run one of the above.

Install wasm-pack command line tool with

    cargo install wasm-pack

Build the project

    wasm-pack build --target web

Serve the web server

    npx serve .

Browse http://localhost:5000/


## Libraries

* wasm-bindgen
