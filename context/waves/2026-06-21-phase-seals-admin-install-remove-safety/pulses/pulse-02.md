# Pulse 02: Implementation

Passed. The admin router now mounts JSON and HTML POST routes for data install
and remove.

Install validates bundled seasons, requires exact `INSTALL <season>`
confirmation, writes embedded regular/playoff bundle files under
`~/.icelines/seasons/<season>/bundle-<season>`, and records SHA-256 hashes in a
manifest. Remove validates season ids, requires exact `REMOVE <season>`
confirmation, and deletes only `~/.icelines/seasons/<season>`.
