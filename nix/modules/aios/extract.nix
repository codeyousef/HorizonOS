# A parser receives only a read-only descriptor from a broker. This unit does
# not enroll roots, index content, or authorize a model/tool request.
{ config, lib, ... }@args:
let
  cfg = config.services.aios.extract;
  package = args.aiosExtract or ((args.aiosPackages or {}).aios-extract or null);
in {
  options.services.aios.extract.enable = lib.mkEnableOption "the bounded descriptor-only document parser";
  config = lib.mkIf cfg.enable {
    assertions = [{ assertion = package != null; message = "Document extraction requires the reviewed aios-extract package."; }];
    environment.systemPackages = lib.optional (package != null) package;
    systemd.user.sockets.aios-extract = {
      description = "Horizon OS private descriptor extraction socket";
      wantedBy = [ "sockets.target" ];
      socketConfig = {
        ListenStream = "%t/aios-extract/worker.sock";
        SocketMode = "0600";
        DirectoryMode = "0700";
        Accept = true;
        MaxConnections = 1;
        Backlog = 1;
        RemoveOnStop = true;
        FlushPending = true;
      };
    };
    systemd.user.services."aios-extract@" = {
      description = "Horizon OS bounded document extraction worker";
      serviceConfig = {
        Type = "exec";
        ExecStart = "${package}/bin/aios-extract --socket-worker";
        StandardInput = "socket";
        StandardOutput = "null";
        StandardError = "journal";
        Restart = "no";
        NoNewPrivileges = true;
        PrivateUsers = true;
        PrivateNetwork = true;
        PrivateTmp = true;
        PrivateDevices = true;
        PrivatePIDs = true;
        ProtectProc = "invisible";
        ProcSubset = "pid";
        ProtectHome = true;
        ProtectSystem = "strict";
        ProtectControlGroups = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectKernelLogs = true;
        ProtectClock = true;
        RestrictAddressFamilies = [ "AF_UNIX" ];
        RestrictNamespaces = true;
        RestrictSUIDSGID = true;
        AmbientCapabilities = "";
        CapabilityBoundingSet = "";
        LockPersonality = true;
        MemoryDenyWriteExecute = true;
        SystemCallArchitectures = "native";
        SystemCallFilter = [ "@system-service" ];
        InaccessiblePaths = [ "-/run" "-/var/lib/aios" "-/nix/var/nix/daemon-socket" ];
        TemporaryFileSystem = "/tmp:rw,nosuid,nodev,noexec,size=64M,mode=0700,uid=%U,gid=%G";
        WorkingDirectory = "/tmp";
        Environment = [ "TMPDIR=/tmp" ];
        UnsetEnvironment = [ "LD_PRELOAD" "LD_LIBRARY_PATH" "LD_AUDIT" "GLIBC_TUNABLES" ];
        UMask = "0077";
        LimitCORE = 0;
        LimitNOFILE = 64;
        LimitFSIZE = "64M";
        MemoryAccounting = true;
        MemoryMax = "512M";
        MemorySwapMax = 0;
        OOMPolicy = "kill";
        CPUAccounting = true;
        # Leave scheduling/period headroom below the ten-second aggregate CPU
        # budget; the deadline covers the entire parser process tree.
        CPUQuota = "90%";
        LimitCPU = 10;
        RuntimeMaxSec = 10;
        TasksMax = 8;
        KillMode = "control-group";
        KillSignal = "SIGKILL";
        TimeoutStopSec = 1;
      };
    };
  };
}
