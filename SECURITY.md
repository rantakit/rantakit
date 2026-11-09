# Security

## Supported versions

Security fixes go to the latest minor release, currently 0.4.x. Until 1.0, each new minor release (such as 0.5) ends support for the one before it. Support from 1.0 onward will be decided later.

## Reporting a problem

Please don't open a public issue. Email **moezrd.dev@gmail.com** with what you found and how to reproduce it.

Rantakit is maintained by one person in their spare time, so replies come as soon as possible, with no fixed deadline.

## What counts

- A file crafted to crash Rantakit or run code when it is opened.
- Any network call besides the update check and the language downloads the user asks for ([ADR 0001](docs/adr/0001-files-never-leave-the-device.md)).
- An Original that gets modified, moved, or overwritten.

Never send real personal documents. If a file triggers the problem, make one that contains nothing private, or describe how to make it.
