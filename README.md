# `Ennona`

## Pointcloud viewer and reconstruction tool for Rust-CV

### Testing locally

`cargo run --release`

On Linux you need to first run `sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev`.

### Compiling for the web

You can compile your app to [WASM](https://en.wikipedia.org/wiki/WebAssembly) and publish it as a web page. For this you need to set up some tools. There are a few simple scripts that help you with this:

``` sh
./setup_web.sh
./build_web.sh
./start_server.sh
open http://127.0.0.1:8080/
```

* `setup_web.sh` installs the tools required to build for web
* `build_web.sh` compiles your code to wasm and puts it in the `docs/` folder (see below)
* `start_server.sh` starts a local HTTP server so you can test before you publish
* Open [127.0.0.1:8080](http://127.0.0.1:8080/) in a web browser to view

The finished web app is found in the `docs/` folder (this is so that you can easily share it with [GitHub Pages](https://docs.github.com/en/free-pro-team@latest/github/working-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site)). It consists of three files:

* `index.html`: A few lines of HTML, CSS and JS that loads your app.
* `cloud_bg.wasm`: What the Rust code compiles to.
* `cloud.js`: Auto-generated binding between Rust and JS.
