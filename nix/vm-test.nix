# The NixOS module put through a Question Set in a VM.
#
# The round trip itself — a Set submitted, a Response posted back, the CLI
# printing it and exiting 0 — is already covered in-process by the crate tests.
# Doing it again here is only worth the VM for what wraps around it, which no
# in-process test can see: a unit that starts itself at boot, a state directory
# systemd creates and hands over, a database that outlives the process that made
# it, a server binary running from the store rather than a working tree, and the
# CLI on `PATH` with nothing set in the environment at all.
#
# Push delivery stays out of scope: it needs the browser vendors' push services,
# which a test VM has no route to. The server's own tests cover what gets sent.
{
  testers,
  git,
  # The flake's NixOS module, which closes over the flake's package — there is
  # no `pkgs.askance` for it to find by name.
  module,
}:

testers.runNixOSTest {
  name = "askance-module";

  nodes.machine = {
    imports = [ module ];

    services.askance.enable = true;

    # The CLI finds its own git through the package's wrapper; this one is here
    # so the test can build the repository the CLI then reads.
    environment.systemPackages = [ git ];

    # The fixtures, in the system closure rather than written from the test
    # script, so the YAML stays YAML and is not two layers of escaping deep.
    environment.etc = {
      "askance-vm-test/set.yaml".text = ''
        # A Question Set as an agent sends it. `project`, `branch` and `diff`
        # are absent on purpose: the CLI derives them from the working
        # directory and overwrites whatever a Set claims.

        title: Does the module hold up in a VM?

        preface: |
          Asked from inside the test VM, so that something has to travel the
          whole way: agent to server to human's device and back again.

        questions:
          - label: Q1
            text: Did the service come up on its own?
            options:
              - n: 1
                text: It did, and its database is in the state directory.
                recommended: true
              - n: 2
                text: It did not.

          - label: Q2
            text: Anything else worth recording?
      '';

      # Two Responses of the same shape, distinguishable in the CLI's output, so
      # a test that answers two Sets can tell which answer reached which agent.
      "askance-vm-test/first-response.yaml".text = ''
        answers:
          - label: Q1
            selected: 1
          - label: Q2
            free_text: The first Set came back to the agent that asked it.

        comment: |
          Posted through the same API the web UI posts through.
      '';

      "askance-vm-test/second-response.yaml".text = ''
        answers:
          - label: Q1
            selected: 1
          - label: Q2
            free_text: The second agent recovered its wait across the restart.

        comment: |
          Answered after the service had been stopped and started under it.
      '';
    };
  };

  testScript = ''
    import re

    # Where the agents' Sets are asked from. The name is distinctive so that
    # finding it in the human's page proves the CLI derived it from *this*
    # directory.
    REPO = "/root/vm-project"

    # Likewise distinctive, and for the same reason: "main" would match the
    # page's own `<main>` and prove nothing.
    BRANCH = "vm-branch"


    def ask(name):
        """Start `askance ask` the way an agent does — in the background, so the
        wait outlives the caller — and leave its stdout, stderr and exit status
        in a directory of its own.

        Nothing is set in the environment: the CLI has only its own default,
        `http://127.0.0.1:8422`, to find the server by.

        Every descriptor of the backgrounded subshell is redirected, the two the
        agent's own output goes to and the rest besides: anything it inherited
        would hold the test driver's pipe open, and the driver waits for that
        pipe to close before it moves on.
        """
        machine.succeed(f"mkdir -p /root/{name}")
        machine.succeed(
            f"( cd {REPO} && askance ask /etc/askance-vm-test/set.yaml"
            f" > /root/{name}/response.yaml 2> /root/{name}/log;"
            f" echo $? > /root/{name}/status )"
            " < /dev/null > /dev/null 2>&1 &"
        )


    def waiting(name):
        """The id of the Set the agent in `name` submitted, once the server has
        taken it.

        Asked of the server rather than of the agent: a wait that goes to plan
        is silent, so the CLI says nothing between submitting a Set and printing
        the Response. The pending list is where an arrival shows, and it carries
        only Sets that are neither answered nor archived — so with one agent
        asking at a time, the Set listed there is that agent's.
        """
        machine.wait_until_succeeds(
            "curl -sf http://127.0.0.1:8422/api/ui/pending | grep -q '\"id\"'"
        )
        listed = machine.succeed("curl -sf http://127.0.0.1:8422/api/ui/pending")
        ids = [int(found) for found in re.findall(r'"id":(\d+)', listed)]
        assert len(ids) == 1, f"expected the one Set {name} asked, got:\n{listed}"

        # The agent has to still be there to be answered: a CLI that submitted
        # and then died would leave the Set pending just the same.
        assert not machine.succeed(
            f"test -e /root/{name}/status && cat /root/{name}/status || true"
        ).strip(), f"{name} exited before its Set was answered"

        return ids[0]


    def answer(set_id, fixture):
        """Post a Response over the API the web UI posts through."""
        machine.succeed(
            "curl -sf -X POST -H 'Content-Type: application/yaml'"
            f" --data-binary @/etc/askance-vm-test/{fixture}"
            f" http://127.0.0.1:8422/api/v1/sets/{set_id}/response"
        )


    def collect(name):
        """Wait for the agent in `name` to finish, insist it exited 0, and take
        what it printed."""
        machine.wait_until_succeeds(f"test -s /root/{name}/status")
        status = machine.succeed(f"cat /root/{name}/status").strip()
        if status != "0":
            log = machine.succeed(f"cat /root/{name}/log")
            raise AssertionError(f"the CLI exited {status}:\n{log}")
        return machine.succeed(f"cat /root/{name}/response.yaml")


    def status_code(url):
        return machine.succeed(
            f"curl -s -o /dev/null -w '%{{http_code}}' '{url}'"
        ).strip()


    start_all()

    with subtest("the service starts itself at boot"):
        # Nothing above this line started it. It is `wantedBy` multi-user.target,
        # so by the time the target is reached it is either running or the
        # module is wrong.
        machine.wait_for_unit("multi-user.target")
        machine.succeed("systemctl is-active --quiet askance.service")
        machine.wait_for_open_port(8422)
        machine.succeed("curl -sf http://127.0.0.1:8422/api/v1/health")

    with subtest("the database is in the state directory, owned by the service"):
        # The server opens the database before it binds, so the open port above
        # already says the file exists; what is asserted here is where it is and
        # whose it is.
        owner = machine.succeed("stat -c %U:%G /var/lib/askance/askance.db").strip()
        assert owner == "askance:askance", f"the database is owned by {owner}"

        directory = machine.succeed("stat -c %U:%G:%a /var/lib/askance").strip()
        assert directory == "askance:askance:750", f"the state directory is {directory}"

    with subtest("the server run from the store serves the viewer built into it"):
        # The viewer is inside the binary rather than beside it, so there is
        # nothing to point the server at and nothing that can be missing from the
        # package — but a build that embedded an empty directory would still start,
        # answer the API, and hand the phone a blank page. This is what catches
        # that, and nothing in-process can: the tests there stand up a fixture site
        # of their own precisely so they need no `pnpm build`.
        document = machine.succeed("curl -sf http://127.0.0.1:8422/")
        assert '<div id="app">' in document, f"no viewer at the root:\n{document}"

        # And the bundles it names are there under the names it names them by,
        # which is the half a document alone does not prove.
        bundles = re.findall(r'(?:href|src)="(/assets/[^"]+)"', document)
        assert bundles, f"the document names no bundle:\n{document}"
        for bundle in bundles:
            machine.succeed(f"curl -sf -o /dev/null http://127.0.0.1:8422{bundle}")

    # The repository an agent always asks from, and which the CLI reads
    # `project`, `branch` and the Diff out of by shelling out to git.
    machine.succeed(f"git -c init.defaultBranch={BRANCH} init -q {REPO}")
    machine.succeed(f"echo committed > {REPO}/tracked.txt")
    machine.succeed(f"git -C {REPO} add -A")
    machine.succeed(
        f"git -C {REPO} -c user.name=Askance -c user.email=vm@askance.invalid"
        " -c commit.gpgsign=false commit -q -m init"
    )
    # Left uncommitted, so the Set carries a Diff as well.
    machine.succeed(f"echo uncommitted > {REPO}/tracked.txt")

    with subtest("an agent's Set is answered through the API and printed by the CLI"):
        ask("first")
        first = waiting("first")

        # The viewer's own namespace is where the Set surfaces — carrying what the
        # CLI derived from the working directory rather than anything the Set
        # claimed. Asked of the API rather than of a page, because the page is
        # drawn in the browser and a `curl` of it is the empty document above.
        pending = machine.succeed("curl -sf http://127.0.0.1:8422/api/ui/pending")
        assert "Does the module hold up in a VM?" in pending, "the Set is not listed"
        assert "vm-project" in pending, "the CLI did not derive the project"
        assert BRANCH in pending, "the CLI did not derive the branch"

        # The third derived field comes with the Set itself rather than with its row.
        detail = machine.succeed(f"curl -sf http://127.0.0.1:8422/api/ui/sets/{first}")
        assert "uncommitted" in detail, "the CLI did not derive the Diff"

        answer(first, "first-response.yaml")
        printed = collect("first")

        assert "The first Set came back" in printed, f"the CLI printed:\n{printed}"
        assert "Posted through the same API" in printed, f"the CLI printed:\n{printed}"
        # stdout is the Response and nothing else — the agent parses it as it
        # stands, so anything the CLI had to say has to have gone to stderr. It
        # says it as a YAML comment, which is what a merged capture stays
        # parseable through; on stdout alone there is nothing to comment.
        assert "# askance:" not in printed, f"the CLI printed:\n{printed}"

    with subtest("a pending Set and its waiting agent survive the service restarting"):
        ask("second")
        pending = waiting("second")

        # Stopped and started rather than restarted: with the server gone for a
        # moment the agent is certain to find it missing, and can be *seen*
        # deciding to come back. A `restart` may slip between two of its polls
        # and prove nothing. `Restart=always` does not fight an explicit stop.
        machine.succeed("systemctl stop askance.service")
        machine.wait_until_succeeds("grep -q retrying /root/second/log")

        machine.succeed("systemctl start askance.service")
        machine.wait_for_open_port(8422)

        # The Set outlived the process that took it: 204 is "still pending, come
        # back", where a database that had not survived would answer 404.
        code = status_code(
            f"http://127.0.0.1:8422/api/v1/sets/{pending}/response?hold=0"
        )
        assert code == "204", f"the pending Set answered {code} after the restart"

        # And so did the Set that was answered before the restart.
        machine.succeed(
            f"curl -sf 'http://127.0.0.1:8422/api/v1/sets/{first}/response?hold=0'"
            " | grep -q 'The first Set came back'"
        )

        # The agent did not fail when the server went away; it reconnects its
        # wait, so answering now still reaches it.
        machine.succeed("test ! -e /root/second/status")
        answer(pending, "second-response.yaml")
        printed = collect("second")

        assert "recovered its wait" in printed, f"the CLI printed:\n{printed}"
  '';
}
