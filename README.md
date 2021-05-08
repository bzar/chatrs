# chatrs

I wanted to try some Rust libraries so I made a chat system. Use `cargo make` to build and run from the target directory.

## src

Contains shared code. `lib.rs` is shared by the server and clients, `client.rs` is additionally shared by clients.

## message_server

Implements a simple chat server using [message-io](https://github.com/lemunozm/message-io).

## web_server

A super simple static file web server using [actix-web](https://github.com/actix/actix-web) to serve web_client.

## web_client

A web client for message_server implemented using [Yew](https://yew.rs).

## cli_client

A terminal client for message_server implemented using [message-io](https://github.com/lemunozm/message-io),
[termion](https://github.com/redox-os/termion) and [tui-rs](https://github.com/fdehau/tui-rs).
