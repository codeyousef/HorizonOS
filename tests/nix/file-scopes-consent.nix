{ pkgs, source, core, consentUi, fixtureUuid, installationUuid }:
let
  probe = pkgs.rustPlatform.buildRustPackage {
    pname = "aios-files-consent-qualification";
    version = "0.1.0";
    src = source;
    cargoLock.lockFile = ../../Cargo.lock;
    cargoBuildFlags = [ "--package" "aios-session" "--example" "native_file_consent" ];
    AIOS_SCOPE_QUALIFICATION_UUID = fixtureUuid;
    nativeBuildInputs = [ pkgs.pkg-config ];
    buildInputs = [ pkgs.sqlite pkgs.systemd ];
    doCheck = false;
    installPhase = ''
      install -Dm755 target/${pkgs.stdenv.hostPlatform.rust.rustcTarget}/release/examples/native_file_consent $out/bin/aios-file-consent-qualification
    '';
  };
  gate = pkgs.writeText "scope-fixture-identity.py" (builtins.readFile ./scope_fixture_identity.py);
  uiPython = pkgs.python3.withPackages (p: [ p.pyatspi p.pygobject3 ]);
  ui = pkgs.writeText "scope-consent-ui.py" (builtins.readFile ./scope_consent_ui.py);
  cacheDiagnostic = pkgs.writeText "scope-cache-syscalls.py" (builtins.readFile ./scope_cache_syscalls.py);
  uiDriver = pkgs.writeShellScript "scope-consent-fixture-ui" ''
    export XDG_RUNTIME_DIR=/run/user/1000
    export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus
    export GI_TYPELIB_PATH=${pkgs.at-spi2-core}/lib/girepository-1.0:${pkgs.glib}/lib/girepository-1.0:${pkgs.gobject-introspection}/lib/girepository-1.0
    exec ${uiPython}/bin/python3 ${ui} "$@"
  '';
in pkgs.testers.runNixOSTest {
  name = "aios-file-scopes-native-consent";
  requiredFeatures.kvm = false;
  node.specialArgs.aiosCore = core;
  nodes.machine = { lib, ... }: {
    imports = [ ../../nix/modules/aios ];
    services.aios.session.enable = true;
    virtualisation.memorySize = 3072;
    virtualisation.cores = 2;
    virtualisation.diskSize = 2048;
    virtualisation.qemu.forceAccel = false;
    virtualisation.qemu.options = [ "-uuid ${fixtureUuid}" "-machine accel=tcg" ];
    users.users.alice = { isNormalUser = true; uid = 1000; };
    services.openssh.enable = true;
    services.desktopManager.plasma6.enable = true;
    services.displayManager.sddm.enable = true;
    services.displayManager.sddm.wayland.enable = true;
    services.displayManager.autoLogin = { enable = true; user = "alice"; };
    services.displayManager.defaultSession = "plasma";
    environment.systemPackages = [ core probe pkgs.python3 pkgs.openssh ];
    environment.etc."aios/scope-consent-native".text = "${consentUi}/bin/.aios-scope-dialog-wrapped\n";
    environment.etc."aios/scope-qualification.json".text = builtins.toJSON {
      schema_version = 1;
      dmi_uuid = fixtureUuid;
      installation_uuid = installationUuid;
      guest_role = "development";
      management_channel = "nixos-test-driver";
    };
    systemd.user.services.aios-scope-consent-test = {
      serviceConfig = {
        ExecStart = "${probe}/bin/aios-file-consent-qualification";
        Type = "oneshot";
        RemainAfterExit = true;
        TimeoutStartSec = 480;
      };
    };
    # Copy the packaged broker protections verbatim. Only service lifecycle
    # and the fixed diagnostic executable differ; no filter is relaxed.
    systemd.user.units."aios-cache-startup-diagnostic.service".text = builtins.replaceStrings
      [ "@EXECUTABLE@" "Type=dbus" "BusName=org.aios.Session1" "Restart=on-failure" "RuntimeDirectory=aios" ]
      [ "${pkgs.python3}/bin/python3 ${cacheDiagnostic}" "Type=oneshot" "" "Restart=no" "RuntimeDirectory=aios-cache-diagnostic" ]
      (builtins.readFile ../../nix/packages/aios-sessiond.service);
    # Synthetic account only: avoid unattended lock during slow TCG checks.
    systemd.tmpfiles.rules = [ "d /home/alice/.config 0700 alice users -" ];
    environment.etc."xdg/kscreenlockerrc".text = "[Daemon]\nAutolock=false\nLockOnResume=false\n";
  };
  testScript = ''
    import json
    import shlex
    import re
    machine.start()
    machine.wait_for_unit("sshd.service")
    gate = "${pkgs.python3}/bin/python3 ${gate}"
    identity = json.loads(machine.succeed(gate + " record"))
    def checked(command):
        return machine.succeed(gate + " verify >/dev/null && ( set -e; " + command + " )")
    user = "su -s /run/current-system/sw/bin/sh alice -c "
    def user_command(command):
        import shlex
        return checked(user + shlex.quote("export XDG_RUNTIME_DIR=/run/user/1000; export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus; " + command))
    def cache_diagnostics():
        # Fixed synthetic metadata only; never print file content, environment
        # or arbitrary process arguments. Retained even when the test fails.
        program = 'import os,json,stat; paths=["/home/alice","/home/alice/.local","/home/alice/.local/state","/home/alice/.local/state/aios","/home/alice/.local/state/aios/files","/home/alice/.local/state/aios/files/content.sqlite3","/home/alice/.local/state/aios/files/content.lock"]; rows=[]; exec("for p in paths:\\n try:\\n  s=os.lstat(p); rows.append(dict(path=p,uid=s.st_uid,mode=oct(stat.S_IMODE(s.st_mode)),kind=stat.S_IFMT(s.st_mode),size=s.st_size))\\n except FileNotFoundError: rows.append(dict(path=p,missing=True))"); print(json.dumps(rows))'
        print("CACHE_STARTUP_METADATA=" + checked("${pkgs.python3}/bin/python3 -c " + shlex.quote(program)))
        print("BROKER_STARTUP_JOURNAL=" + user_command("journalctl --user -u aios-sessiond.service --no-pager -n 12"))
        try:
            user_command("systemctl --user start aios-cache-startup-diagnostic.service")
        except Exception:
            print("CACHE_SYSCALL_DIAGNOSTIC_UNIT_FAILED")
        print("CACHE_SYSCALL_JOURNAL=" + user_command("journalctl --user -u aios-cache-startup-diagnostic.service --no-pager -n 12"))
    checked("install -d -o alice -g users -m 0700 /home/alice/scope-consent /home/alice/Desktop /home/alice/Documents /home/alice/Downloads /home/alice/Projects /home/alice/Documents/.local/state/aios")
    checked("printf 'consented installed content' > /home/alice/Documents/note.txt; printf excluded > /home/alice/Documents/.env; printf excluded > /home/alice/Documents/id_ed25519; printf excluded > /home/alice/Documents/wallet.dat; printf excluded > /home/alice/Documents/.local/state/aios/private.txt; printf '%s%s' '-----BEGIN ' 'OPENSSH PRIVATE KEY-----' > /home/alice/Documents/renamed.txt; chown -R alice:users /home/alice/Documents; ln -s /etc/passwd /home/alice/Documents/escape.txt")
    machine.wait_until_succeeds(gate + " verify >/dev/null && " + user + "'export XDG_RUNTIME_DIR=/run/user/1000; export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus; systemctl --user is-active plasma-kwin_wayland.service'", timeout=180)
    try:
        machine.wait_until_succeeds(gate + " verify >/dev/null && " + user + "'export XDG_RUNTIME_DIR=/run/user/1000; export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus; systemctl --user is-active aios-sessiond.service'", timeout=30)
    except Exception:
        cache_diagnostics()
        raise
    # Managed public startup must refuse an unprotected launch, even when the
    # same account supplies a valid runtime and bus. This touches only a new
    # bounded fixture process; the installed service must remain active.
    unmanaged = 'from pathlib import Path; import json; s=dict(line.split(":",1) for line in Path("/proc/self/status").read_text().splitlines() if ":" in line); print(json.dumps({"nnp":s["NoNewPrivs"].strip()}))'
    assert json.loads(user_command("${pkgs.python3}/bin/python3 -c " + shlex.quote(unmanaged)))["nnp"] == "0"
    user_command("set -e; if timeout 5 ${core}/bin/aios-sessiond >/home/alice/scope-consent/unmanaged-start.log 2>&1; then exit 1; fi; grep -F 'provider privilege restriction absent' /home/alice/scope-consent/unmanaged-start.log")
    user_command("systemctl --user is-active aios-sessiond.service")
    session = [line.split()[0] for line in checked("loginctl list-sessions --no-legend --no-pager").splitlines() if len(line.split()) >= 3 and line.split()[1:3] == ["1000", "alice"]]
    selected = []
    for item in session:
        assert re.fullmatch(r"[A-Za-z0-9_-]{1,128}", item)
        properties = dict(line.split("=", 1) for line in checked("loginctl show-session " + item + " --no-pager -p User -p Type -p Active -p Remote -p State -p Class").splitlines())
        if properties == {"User":"1000","Type":"wayland","Active":"yes","Remote":"no","State":"active","Class":"user"}:
            selected.append(item)
    assert len(selected) == 1
    broker_pid = int(user_command("systemctl --user show aios-sessiond.service --property=MainPID --value").strip())
    assert broker_pid > 0
    session_record = json.dumps({"session_id": selected[0], "broker_pid": broker_pid})
    checked("install -d -m 0755 /run/aios-scope-consent; printf %s " + shlex.quote(session_record) + " > /run/aios-scope-consent/session.json; chmod 0644 /run/aios-scope-consent/session.json")
    user_command("systemctl --user start --no-block aios-scope-consent-test.service")
    machine.wait_for_file("/home/alice/scope-consent/cancel-pending", timeout=90)
    cancel_view = json.loads(user_command("${uiDriver} inspect"))
    checked("true")
    machine.screenshot("file-root-cancel")
    checked("touch /home/alice/scope-consent/cancel-visible")
    machine.wait_for_file("/home/alice/scope-consent/cancel-refused", timeout=45)
    machine.wait_for_file("/home/alice/scope-consent/changed-pending", timeout=45)
    changed_view = json.loads(user_command("${uiDriver} inspect"))
    checked("mv /home/alice/Documents /home/alice/Documents.changed; install -d -o alice -g users -m 0700 /home/alice/Documents")
    machine.wait_for_file("/home/alice/scope-consent/changed-refused", timeout=45)
    checked("rmdir /home/alice/Documents; mv /home/alice/Documents.changed /home/alice/Documents; touch /home/alice/scope-consent/root-restored")
    machine.wait_for_file("/home/alice/scope-consent/allow-pending", timeout=45)
    allow_view = json.loads(user_command("${uiDriver} inspect"))
    checked("true")
    machine.screenshot("file-root-allow")
    allow_view = json.loads(user_command("${uiDriver} allow"))
    machine.wait_for_file("/home/alice/scope-consent/cache-populated", timeout=45)
    def cache_counts():
        program = 'import sqlite3,json; c=sqlite3.connect("file:/home/alice/.local/state/aios/files/content.sqlite3?mode=ro",uri=True); print(json.dumps([c.execute("SELECT count(*) FROM "+t).fetchone()[0] for t in ("documents","chunks","previews","snippets")]))'
        return json.loads(user_command("${pkgs.python3}/bin/python3 -c " + shlex.quote(program)))
    assert cache_counts() == [1,1,1,1]
    checked("touch /home/alice/scope-consent/cache-inspected")
    machine.wait_for_file("/home/alice/scope-consent/result.json", timeout=45)
    result = json.loads(checked("cat /home/alice/scope-consent/result.json"))
    assert result["identity"] == identity
    assert cache_counts() == [0,0,0,0]
    result["stored_material_counts_verified"] = True
    result["native_ui_views"] = [cancel_view, changed_view, allow_view]
    result["native_ui_consent_qualified"] = True
    result["unmanaged_startup_refused"] = True
    with open(machine.out_dir / "scope-consent.json", "w") as report:
        json.dump(result, report, indent=2)
  '';
}
