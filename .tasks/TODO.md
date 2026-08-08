# Update Notice

A server that has been running since a newer release was published shows the
human an Update Notice — a banner above the pending list, naming the version
and linking the updating instructions. Nothing is installed on anyone's
behalf, and an env var turns the check off entirely.

The poll is server-side rather than in the browser: every device asking GitHub
directly would have the whole tailnet phoning home, and the viewer should not
depend on GitHub being reachable. It runs at startup and daily thereafter, so
a server restarted just after a release notices immediately, and the verdict
is held in memory — a failed poll costs nothing and is retried next cycle.

Roadmap stage: [05: Update Notice](docs/roadmaps/public-release/05-update-notice.md)

## Tasks

- [x] 01: The update poll and what it answers — [details](01-update-poll.md)
- [x] 02: The Update Notice banner — [details](02-notice-banner.md)
