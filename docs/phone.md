# On your phone

The point of putting Askance on a phone is that a Question Set reaches you
without the pending list being open. That needs a push notification, and a push
notification needs HTTPS: service workers, the Push API and
`Notification.requestPermission` are all withheld outside a secure context, and
`http://` over a tailnet is not one — only `localhost` gets the exemption. So
this guide is HTTPS first, and everything else follows from it.

## 1. Put `tailscale serve` in front of it

`tailscale serve` terminates TLS with your tailnet's `ts.net` certificate and
proxies to the plain HTTP the server is already binding. Nothing about the
server changes: it stays on loopback, and the proxy is the only thing that
listens on the tailnet.

```console
$ tailscale serve --bg 8422
Available within your tailnet:

https://your-host.your-tailnet.ts.net/
|-- proxy http://127.0.0.1:8422

Serve started and running in the background.
To disable the proxy, run: tailscale serve --https=443 off
```

That URL is the one to open on the phone. It needs MagicDNS and HTTPS
certificates enabled for the tailnet — both are switches in the admin console,
under DNS — and `tailscale serve` says so if they are not.

`--bg` is what makes it persist: the configuration is stored in `tailscaled`'s
own preferences, so it survives a reboot and comes back with the daemon. Check
what is in force, or take it down again:

```console
$ tailscale serve status
https://your-host.your-tailnet.ts.net (tailnet only)
|-- / proxy http://127.0.0.1:8422

$ tailscale serve reset
```

This stays inside the tailnet. `tailscale funnel`, which is the sibling command
that would put the same service on the public internet, is not what you want
here: there is no app-level auth in Askance, and the tailnet is the whole
perimeter.

The CLI has no reason to go through the proxy — an agent runs on the same host
as the server and keeps talking to `http://127.0.0.1:8422`. Only the browser
needs the HTTPS URL.

## 2. Install it

On the phone, open the `ts.net` URL and add it to the home screen.

- **iOS/iPadOS** (16.4 or later): Safari, Share, **Add to Home Screen**. This
  is not optional — iOS gives Web Push only to a web app launched from the home
  screen, so in a Safari tab there is nothing to turn on, and the control on
  the pending list says notifications are unavailable. Open it from the home
  screen icon afterwards.
- **Android**: Chrome offers **Install app** from its menu. Push works in the
  tab too, but the installed app is what gets you an icon and no browser
  chrome.

Either way it opens standalone, without the address bar, on the pending list.

## 3. Turn notifications on

At the top of the pending list is a line saying where this device stands, with
one button. Tap **Turn on for this device** and answer the browser's permission
prompt.

This is per device, and it is read out of the browser on every load rather than
remembered — the phone being subscribed says nothing about the laptop, and an
app reopened a week later says what is actually true of it. **Turn off for this
device** is the way back. If it says notifications are *blocked*, the browser
has been told no and will not ask again: the way out is that browser's site
settings, not another tap.

On a Chromium browser that de-Googles — **Brave** above all — the tap can fail
with *Registration failed - push service error*. Chromium has no push transport
but Google's, and Brave ships with it switched off, so the subscribe is refused
inside the browser before this server hears about it. Turn on **Use Google
services for push messaging** under `brave://settings/privacy` and restart the
browser. That is the de-Googling trade this browser exists to offer, so it is
yours to make rather than something Askance can work around: Safari and Chrome
are unaffected, and so is Brave on Android once the same setting is on.

From then on, one notification per arriving Set — titled with the Set's own
title, with the project underneath it — and tapping it opens that Set, in the
Askance already on screen if there is one. There are no reminders: a Set that
goes unanswered is not notified about twice.

## The long waits, through the proxy

Two things here stay open much longer than a page load, and both go through
`tailscale serve` once the phone is the device answering: the CLI's wait, which
holds a request for up to a minute before reopening it, and the pending list's
ten-second refetch. Both already survive a dropped connection, so the question
was never whether the proxy breaks them but whether it makes them work harder
than they need to. It does not. Measured against tailscale 1.90.9:

- A full hold — 60 seconds, the server's ceiling — comes back `204` at 60.0s.
  The proxy neither cuts it short nor shortens the window it was asked for.
- A hold answered five seconds in comes back `200` at 5.0s. Nothing is
  buffered, which is what lets the phone's **Submit** wake a waiting agent
  immediately instead of at the end of whichever hold happened to be open.
- `askance ask` pointed at the `ts.net` URL and left for 75 seconds — three of
  its own 30-second holds — printed the Response and exited 0, having said
  nothing on stderr beyond the line it opens with. The reconnections are there
  and are invisible, which is the whole of what was asked of them.
- The pending list refetches as it does locally: a Set submitted while the
  installed app sits open arrives on the list without a touch.

The client end is HTTP/2 and the loopback hop is plain HTTP/1.1. The `ts.net`
certificate is publicly trusted, so nothing needs adding to a trust store —
which is the other reason this works on a phone at all.

## The one thing that leaves the tailnet

Web Push is delivered by the browser vendors' push services — Apple's, Google's
— so **the server needs outbound internet to send a notification**, even though
its inbound surface stays tailnet-only. That asymmetry is the whole of it:
nothing reaches Askance from outside the tailnet, and the only thing Askance
reaches out to is the push service for the device it is notifying, carrying an
encrypted payload it cannot read.

The VAPID keypair that signs those pushes is generated on first run and stored
in `askance.db`. There is no key ceremony and nothing to configure; a push
service that cannot be reached costs a notification, never a Question Set.
