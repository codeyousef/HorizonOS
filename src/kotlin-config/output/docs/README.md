# HorizonOS Configuration

This configuration was generated from a HorizonOS Kotlin DSL file.

## System Information

- **Hostname**: horizonos-minimal
- **Timezone**: UTC
- **Locale**: en_US.UTF-8


## Deployment

To deploy this configuration:

```bash
cd scripts
sudo ./deploy.sh
```

## Files Generated

### JSON Files
- `json/config.json`
- `ostree/manifest.json`

### YAML Files
- `yaml/config.yaml`

### Systemd Units
- `systemd/horizonos-config.service`
- `systemd/horizonos-update.timer`

### Shell Scripts
- `scripts/deploy.sh`
- `scripts/system-config.sh`
- `scripts/package-manager.sh`
- `scripts/service-manager.sh`
- `scripts/user-manager.sh`
- `scripts/repository-config.sh`
- `ostree/build-ostree.sh`

### Ansible Playbooks
- `ansible/horizonos-playbook.yml`

### Docker Files
- `docker/Dockerfile`

