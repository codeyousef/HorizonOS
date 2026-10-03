"""Conservative fresh-VM defaults from read-only host discovery."""
from .errors import DevctlError, ExitCode

GIB = 1024**3


def recommend(report):
    cpu = report.get("logical_cpus")
    available = report.get("memory", {}).get("available_bytes")
    free = report.get("disk", {}).get("free_bytes")
    if any(type(value) is not int or value <= 0 for value in (cpu, available, free)):
        raise DevctlError(ExitCode.UNMET_PREREQUISITE, "RESOURCE_DISCOVERY_REQUIRED", "Fresh VM sizing requires measured CPUs, available RAM and free storage")
    memory_reserve = max(2 * GIB, available // 4)
    memory_gib = min(16, (available - memory_reserve) // GIB)
    disk_gib = min(96, (free - 8 * GIB) // GIB)
    if memory_gib < 4 or disk_gib < 48:
        raise DevctlError(ExitCode.UNMET_PREREQUISITE, "INSUFFICIENT_DEFAULT_VM_RESOURCES", "Fresh development defaults require at least 4 GiB guest RAM and 48 GiB disk after host reserves")
    return {"vcpus": min(8, max(1, cpu // 2)), "memory_mib": memory_gib * 1024,
            "disk_gib": disk_gib, "host_memory_reserve_bytes": memory_reserve,
            "host_disk_reserve_bytes": 8 * GIB}
