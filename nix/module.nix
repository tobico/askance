# Askance as a systemd service: the server under its own user, with the CLI on
# every user's `PATH` so an agent working on the box can just call `askance`.
#
# The package is the flake's, so the module is a function of it rather than of
# `pkgs` — nothing here is in nixpkgs to be found by name.
self:

{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.askance;

  # systemd creates and owns this, and the database defaults inside it. Named
  # once because the sandbox, the working directory and that default all say it.
  stateDir = "/var/lib/askance";
in

{
  options.services.askance = {
    enable = lib.mkEnableOption "Askance, through which coding agents put questions to a human" // {
      description = ''
        Whether to run the Askance server as a system service, with the CLI on
        every user's `PATH`.

        The server binds the loopback interface and speaks plain HTTP.
        Reaching the web UI from a phone means HTTPS, which is
        `tailscale serve`'s job in front of it and stays host-level
        configuration — the Askance README's "On your phone" section has the
        invocation, and this module deliberately keeps no second copy of it.
      '';
    };

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.askance;
      defaultText = lib.literalExpression "askance.packages.\${system}.askance";
      description = ''
        The Askance package to run. One derivation carries both halves: the
        server this service starts and the CLI it puts on `PATH`.
      '';
    };

    listen = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1:8422";
      example = "0.0.0.0:8422";
      description = ''
        Address and port the server binds, as `ASKANCE_LISTEN`.

        The default is the server's own: loopback, which is what
        `tailscale serve` proxies to. Binding a tailnet address instead reaches
        other devices directly, but over plain HTTP — which rules out the push
        notifications, since a service worker needs a secure context. The
        Askance README's "On your phone" section is where that story lives.

        The CLI's own default is `http://127.0.0.1:8422`, so a host that changes
        the port here has to set `ASKANCE_SERVER` for the agents alongside it.
      '';
    };

    database = lib.mkOption {
      type = lib.types.path;
      default = "${stateDir}/askance.db";
      defaultText = lib.literalExpression ''"${stateDir}/askance.db"'';
      description = ''
        SQLite file, as `ASKANCE_DATABASE`. Created, with its parent directory,
        on first run; it holds the Question Sets, the Archive, the push
        subscriptions and the VAPID keypair, so it is the whole of the service's
        state.

        The default is the server's own filename inside the service's state
        directory. Pointing it elsewhere means the sandbox has to be opened up
        for that path, which this module does by directory — so the directory
        has to exist, even though the file need not.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    # Both binaries land on `PATH`; an agent runs the CLI, and `askance-server
    # --help` is how a human finds out what this unit is passing it.
    environment.systemPackages = [ cfg.package ];

    users.users.askance = {
      isSystemUser = true;
      group = "askance";
      description = "Askance server";
    };
    users.groups.askance = { };

    systemd.services.askance = {
      description = "Askance — questions from coding agents to a human";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      serviceConfig = {
        ExecStart = lib.escapeShellArgs [
          "${cfg.package}/bin/askance"
          "serve"
          "--listen"
          cfg.listen
          "--database"
          "${cfg.database}"
        ];

        User = "askance";
        Group = "askance";

        # systemd makes the directory and hands it over already owned; the
        # service never creates it, and it survives a restart with the database
        # in it. Relative paths the server is given resolve here too.
        StateDirectory = "askance";
        StateDirectoryMode = "0750";
        WorkingDirectory = stateDir;

        # An agent is blocked on an answer whenever the server is down, so come
        # back rather than sit in a failed state.
        Restart = "always";
        RestartSec = "5s";

        # Hardening. Two things it must not break: SQLite in WAL mode, which
        # creates `-wal` and `-shm` beside the database and so needs a
        # read-write directory rather than just a writable file; and outbound
        # HTTPS to the browser vendors' push services, whose addresses cannot be
        # enumerated ahead of time — which is why there is no `IPAddressAllow`
        # here.
        CapabilityBoundingSet = [ "" ];
        NoNewPrivileges = true;
        PrivateDevices = true;
        PrivateTmp = true;
        PrivateUsers = true;
        ProtectClock = true;
        ProtectControlGroups = true;
        ProtectHome = true;
        ProtectHostname = true;
        ProtectKernelLogs = true;
        ProtectKernelModules = true;
        ProtectKernelTunables = true;
        ProtectProc = "invisible";
        ProcSubset = "pid";
        ProtectSystem = "strict";
        # AF_UNIX is the journal's socket and AF_NETLINK is what glibc's
        # resolver asks which interfaces exist over — neither is the server
        # reaching anywhere.
        RestrictAddressFamilies = [
          "AF_INET"
          "AF_INET6"
          "AF_UNIX"
          "AF_NETLINK"
        ];
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        LockPersonality = true;
        MemoryDenyWriteExecute = true;
        SystemCallArchitectures = "native";
        SystemCallFilter = [
          "@system-service"
          "~@privileged"
          "~@resources"
        ];
        UMask = "0077";

        # `ProtectSystem = "strict"` leaves the state directory writable and
        # nothing else, so a database put elsewhere needs its directory saying
        # so. Under the state directory this would be redundant.
        ReadWritePaths = lib.optional (!lib.hasPrefix "${stateDir}/" "${cfg.database}") (
          builtins.dirOf "${cfg.database}"
        );
      };
    };
  };
}
