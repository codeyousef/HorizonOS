# Actual model service in a disposable two-user desktop acceptance image.
# No mock model, alternate daemon mode, or model privilege bypass is enabled.
{ ... }: {
  services.aios.users = [ "dev" "tester" ];
  services.aios.model.enable = true;
  environment.etc."aios/model-test-profile".text = "installed-normal-cpu-model-v1\n";
}
