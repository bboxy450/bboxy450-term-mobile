# bboxy450-term-mobile: working instructions

The rules of `vendor/bboxy450-term/AGENTS.md` apply here unchanged. They cover:
the stand, quiet operation, signed commits as Boxy <bboxy450@gmail.com>,
Respectful Slop attribution, mandatory formal verification for stateful code,
unattended tests, and the Makefile as the only interface. Tasks and specs live
in the bboxy450-term knowledge base (`.bboxy/`), not here.

Rules specific to this repository:
- The UI kit, the protocol and the client core belong in `bboxy450-term`.
  Change them there and move the pin with `make bump-term`; never patch them
  under `vendor/`.
- Simulators and emulators only. No device signing, Apple or Google account,
  or store upload until a release brief authorizes it (round 4 §4).
- Smoke tests capture the simulator or emulator, never the operator's desktop.
