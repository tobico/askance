# 05. Serving over Tailscale, and the install

## What to build

The README section that gets Askance onto a phone: how to put it behind
`tailscale serve` for HTTPS, how to install it, and how to turn notifications on.

Cover the parts that will otherwise cost an evening each: that HTTPS is what
service workers and Web Push require and `tailscale serve` supplies it with the
`ts.net` certificate; that on iOS push only works once the app has been added to
the home screen, so installing is not optional there; and that the server needs
outbound internet to reach the push services even though nothing inbound leaves
the tailnet.

Confirm, against a real `tailscale serve`, that the two long-lived paths still
behave — the CLI's wait (which holds a connection up to a minute and reconnects)
and the pending list's ten-second refetch. Both are built to survive a dropped
connection, so the point is to record what actually happens rather than to fix
anything. If the proxy does cut or buffer them, write down what it does.

The README's Status section is also stale: it still says submitting is not wired
up and has `curl` playing the human's part, which stages 02 and 03 have since
made untrue. Bring it up to what the tool now does.

## Acceptance criteria

- [ ] The README carries the `tailscale serve` invocation, verified by running it,
      with the resulting URL and how to make it persist
- [ ] The install and enable-notifications steps are written as a walkthrough, with
      the iOS home-screen requirement called out
- [ ] The outbound-internet note sits with the push documentation, distinguishing
      it from the tailnet-only inbound surface
- [ ] The CLI's wait and the pending list's refetch are exercised through
      `tailscale serve` and their behaviour recorded
- [ ] The Status section describes the tool as it now is, and the quickstart's
      human step is the web UI rather than `curl`
