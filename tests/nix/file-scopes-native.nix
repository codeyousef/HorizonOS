{ pkgs, source, fixtureUuid, installationUuid }:
let
  probe = pkgs.rustPlatform.buildRustPackage {
    pname = "aios-files-native-qualification";
    version = "0.1.0";
    src = source;
    cargoLock.lockFile = ../../Cargo.lock;
    cargoBuildFlags = [ "--package" "aios-files" "--example" "native_boundaries" ];
    AIOS_SCOPE_QUALIFICATION_UUID = fixtureUuid;
    nativeBuildInputs = [ pkgs.pkg-config ];
    buildInputs = [ pkgs.sqlite ];
    doCheck = false;
    installPhase = ''
      runHook preInstall
      install -Dm755 target/${pkgs.stdenv.hostPlatform.rust.rustcTarget}/release/examples/native_boundaries $out/bin/aios-file-scope-qualification
      runHook postInstall
    '';
  };
  gate = pkgs.writeText "scope-fixture-identity.py" (builtins.readFile ./scope_fixture_identity.py);
in pkgs.testers.runNixOSTest {
  name = "aios-file-scope-native-boundaries";
  requiredFeatures.kvm = false;
  nodes.machine = {
    virtualisation.memorySize = 768;
    virtualisation.cores = 2;
    virtualisation.diskSize = 1024;
    virtualisation.qemu.forceAccel = false;
    # Override the generated machine's kvm:tcg accelerator property. QEMU
    # rejects mixing that property with a separate -accel option.
    virtualisation.qemu.options = [ "-uuid ${fixtureUuid}" "-machine accel=tcg" ];
    users.users.alice = { isNormalUser = true; uid = 1000; };
    users.users.bob = { isNormalUser = true; uid = 1001; };
    services.openssh.enable = true;
    environment.systemPackages = [ probe pkgs.python3 pkgs.openssh ];
    environment.etc."aios/scope-qualification.json".text = builtins.toJSON {
      schema_version = 1;
      dmi_uuid = fixtureUuid;
      installation_uuid = installationUuid;
      guest_role = "development";
      management_channel = "nixos-test-driver";
    };
    systemd.services.aios-scope-probe = {
      serviceConfig = {
        User = "alice";
        ExecStart = "${probe}/bin/aios-file-scope-qualification alice";
        Type = "oneshot";
        RemainAfterExit = true;
        TimeoutStartSec = 300;
      };
    };
  };
  testScript = ''
    import json
    machine.start()
    machine.wait_for_unit("sshd.service")
    gate = "${pkgs.python3}/bin/python3 ${gate}"
    # Bootstrap trust comes from this driver-owned, UUID-bound console, never
    # ssh-keyscan. All following mutations recheck the complete recorded target.
    identity = json.loads(machine.succeed(gate + " record"))
    def checked(command):
        return machine.succeed(gate + " verify >/dev/null && ( set -e; " + command + " )")
    checked("install -d -o alice -g users -m 0700 /home/alice/scope-probe /home/alice/Documents /home/alice/Documents/mounted /home/alice/replacement")
    checked("printf 'نص آمن / safe text\\n' > /home/alice/Documents/note.txt; printf old > /home/alice/Documents/mounted/note.txt; printf foreign > /home/alice/Documents/foreign.txt; chown alice:users /home/alice/Documents/note.txt /home/alice/Documents/mounted/note.txt; chown bob:users /home/alice/Documents/foreign.txt; chmod 0644 /home/alice/Documents/foreign.txt; ln -s /etc/passwd /home/alice/Documents/escape.txt")
    checked("systemctl start --no-block aios-scope-probe.service")
    machine.wait_for_file("/home/alice/scope-probe/ready", timeout=180)
    checked("mount -t tmpfs -o size=1m tmpfs /home/alice/Documents/mounted; chown alice:users /home/alice/Documents/mounted; printf new > /home/alice/Documents/mounted/note.txt; chown alice:users /home/alice/Documents/mounted/note.txt; touch /home/alice/scope-probe/mounted")
    machine.wait_for_file("/home/alice/scope-probe/mount-denied", timeout=90)
    checked("mount --bind /home/alice/replacement /home/alice/Documents; touch /home/alice/scope-probe/root-replaced")
    machine.wait_for_unit("aios-scope-probe.service")
    assert checked("systemctl show aios-scope-probe.service -p ExecMainStatus --value").strip() == "0"
    assert "actual-foreign-uid-cache-denied" in checked("su -s /run/current-system/sw/bin/sh bob -c '${probe}/bin/aios-file-scope-qualification bob'")
    result = json.loads(checked("cat /home/alice/scope-probe/result.json"))
    result["actual_foreign_uid_cache_denied"] = True
    assert result["identity"] == identity
    for key in ["preconsent_denied", "foreign_owned_readable_file_denied", "symlink_escape_denied", "new_mount_denied", "retained_handle_mount_change_denied", "replaced_root_blocks_cached_snippet"]:
        assert result[key] is True
    assert result["persistent_purge"]["cached_records_purged"] == 3
    assert result["persistent_purge"]["purge_pending"] is False
    checked("umount /home/alice/Documents; umount /home/alice/Documents/mounted; test -f /home/alice/Documents/note.txt")
    with open(machine.out_dir / "scope-boundaries.json", "w") as report:
        json.dump(result, report, indent=2)
  '';
}
