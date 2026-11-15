# Contributing

The application is experimental. Preserve the separation between the UI, the Rust core and the future privileged helper. Do not implement physical disk writes until the target OS has a validated privilege elevation and secure IPC design.

Run the checks in the README before proposing changes. Automated write tests must use temporary files or verified isolated virtual devices created by the test; never use a real disk. Manual acceptance on a dedicated USB drive is separate from automated testing.

Implement disk formats using specifications and OS documentation. Do not read or copy the source code of Rufus, Ventoy or UltraISO. Check dependency and distribution licenses before adding packages or external tools.
