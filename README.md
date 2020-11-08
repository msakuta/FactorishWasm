# FactorishWasm

A port of [FactorishJS](https://github.com/msakuta/FactorishJS) to Wasm/Rust (and a bit of HTML5+JavaScript).

Try it now!
https://msakuta.github.io/FactorishWasm/index.html


## Features

This project is a demonstration that how plain HTML5 and JavaScript can be used to create a game
with complexity like the great game [Factorio](https://store.steampowered.com/app/427520/Factorio/).


## Prerequisites

This game uses JavaScript and WebAssembly (Wasm), so you need a browser with WebAssembly support.
Most modern browser support it nowadays.



## How to build and run

Install

* Cargo >1.40
* npm

Install wasm-pack command line tool with

    cargo install wasm-pack

Build the project

    wasm-pack build --target web

Serve the web server

    npx serve .

Browse http://localhost:5000/


## Libraries

* wasm-bindgen
