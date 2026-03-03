import atexit
import logging
import os
import shutil
import sys
from datetime import datetime

LOG_DIR_NAME = "logs"
LATEST_LOG_NAME = "latest.log"
ERROR_LOG_NAME = "error.log"

_latest_log_path = None
_finalized = False
_atexit_registered = False


def _get_log_dir(base_dir: str | None = None) -> str:
    root_dir = base_dir or os.getcwd()
    return os.path.join(root_dir, LOG_DIR_NAME)


def _build_archive_path(log_dir: str) -> str:
    timestamp = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
    candidate = os.path.join(log_dir, f"{timestamp}.log")
    suffix = 1

    while os.path.exists(candidate):
        candidate = os.path.join(log_dir, f"{timestamp}_{suffix}.log")
        suffix += 1

    return candidate


def setup_logging(log_file: str = "log.txt") -> logging.Logger:
    """Initialize logging to logs/latest.log + stdout."""
    global _latest_log_path, _finalized, _atexit_registered

    log_dir = _get_log_dir()
    os.makedirs(log_dir, exist_ok=True)
    _latest_log_path = os.path.join(log_dir, LATEST_LOG_NAME)
    _finalized = False

    root_logger = logging.getLogger()
    for handler in list(root_logger.handlers):
        root_logger.removeHandler(handler)
        try:
            handler.close()
        except Exception:
            pass

    file_handler = logging.FileHandler(_latest_log_path, mode='w', encoding='utf-8')
    stream_handler = logging.StreamHandler(sys.stdout)

    logging.basicConfig(
        level=logging.DEBUG,
        format='[%(asctime)s] %(message)s',
        datefmt='%Y-%m-%d %H:%M:%S',
        handlers=[file_handler, stream_handler]
    )

    if not _atexit_registered:
        atexit.register(finalize_logging)
        _atexit_registered = True

    return logging.getLogger()


def finalize_logging() -> str | None:
    """Close handlers and archive logs/latest.log to a timestamped log file."""
    global _finalized

    if _finalized:
        return None

    _finalized = True
    logging.shutdown()

    if not _latest_log_path or not os.path.exists(_latest_log_path):
        return None

    archive_path = _build_archive_path(os.path.dirname(_latest_log_path))
    os.replace(_latest_log_path, archive_path)
    return archive_path


def capture_error_log() -> str | None:
    """Copy logs/latest.log to logs/error.log for crash reporting."""
    if not _latest_log_path:
        return None

    root_logger = logging.getLogger()
    for handler in root_logger.handlers:
        try:
            handler.flush()
        except Exception:
            pass

    if not os.path.exists(_latest_log_path):
        return None

    error_log_path = os.path.join(os.path.dirname(_latest_log_path), ERROR_LOG_NAME)
    shutil.copyfile(_latest_log_path, error_log_path)
    return error_log_path
