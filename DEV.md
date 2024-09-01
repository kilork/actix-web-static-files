# Developer's handbook

Here I collect info about project ideas and maintenance.

## Documentation

Because `README.md` and `lib.rs` contains almost the same content and because of many external examples and related complexity to keep it all in sync - I increase complexity even more by generating documentation from handlebars templates using my own developed utility [handlebars-magic](https://github.com/rust-utility/handlebars-magic).

To update `README.md` and `lib.rs`:

    handlebars-magic templates .
