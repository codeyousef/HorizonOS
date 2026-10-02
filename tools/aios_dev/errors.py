from enum import IntEnum


class ExitCode(IntEnum):
    SUCCESS = 0
    INVALID_INPUT = 2
    UNMET_PREREQUISITE = 3
    TARGET_MISMATCH = 4
    AUTHORIZATION_NEEDED = 5
    OPERATION_FAILURE = 6
    TIMEOUT = 7
    VERIFICATION_FAILURE = 8
    UNSUPPORTED_CAPABILITY = 9


class DevctlError(Exception):
    def __init__(self, exit_code: ExitCode, code: str, message: str):
        super().__init__(message)
        self.exit_code = exit_code
        self.code = code
