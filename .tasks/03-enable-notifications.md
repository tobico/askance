# 03. Enable notifications on this device

## What to build

A control on the pending list that turns notifications on for the device it is
tapped on, and says plainly where that device stands.

Tapping it asks for notification permission, subscribes through the service
worker's push manager with the public key from task 02, and sends the resulting
subscription to the server. Per device, like the drafts in `localStorage`: the
phone being subscribed says nothing about the laptop.

The control has to be honest about the states the browser can leave it in —
subscribed, not subscribed, permission denied, and unsupported (no service worker,
or the page is not in a secure context). Denied is a dead end the browser will not
re-prompt out of, so say so rather than offering a tap that cannot work. On load,
read the current subscription from the browser rather than assuming: an installed
app reopened a week later must not offer to enable what is already enabled.

Nothing is sent yet — that is task 04. What this task delivers is a phone that has
asked to be told.

## Acceptance criteria

- [ ] Tapping the control on the phone prompts for permission and, once granted,
      lands exactly one subscription in the database
- [ ] Reloading the page shows the device as already subscribed, without
      re-prompting or storing a second subscription
- [ ] Declining the prompt leaves the control saying notifications are blocked for
      this device, with no subscription stored
- [ ] A browser with no service worker, or a page served over plain HTTP from a
      non-localhost address, shows the control as unavailable instead of failing
      on a tap
- [ ] Turning notifications off removes this device's subscription, and the
      control offers to enable them again
