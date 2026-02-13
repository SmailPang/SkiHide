import sys
import ctypes
import logging
import tkinter as tk

from skihide.utils.privilege import ensure_admin_or_exit
from skihide.utils.logging_setup import setup_logging
from skihide.app import SkiHideApp

def _setup_console_visibility(is_debug: bool, logger: logging.Logger):
    if sys.platform != 'win32':
        return
    try:
        console_window = ctypes.windll.kernel32.GetConsoleWindow()
        if is_debug:
            if console_window:
                ctypes.windll.user32.ShowWindow(console_window, 1)
            else:
                ctypes.windll.kernel32.AllocConsole()
                sys.stdout = open('CONOUT$', 'w', encoding='utf-8')
                sys.stderr = open('CONERR$', 'w', encoding='utf-8')
            logger.info("开发模式启动 - 控制台窗口已显示")
        else:
            if console_window:
                ctypes.windll.user32.ShowWindow(console_window, 0)
                logger.info("隐藏控制台窗口")
    except Exception:
        pass

def main():
    try:
        ensure_admin_or_exit()
        logger = setup_logging("log.txt")

        is_debug = any(a.upper() in ["-DEBUG", "--DEBUG", "/DEBUG"] for a in sys.argv[1:])
        is_silent = any(a.upper() in ["-SILENT", "--SILENT", "/SILENT"] for a in sys.argv[1:])
        _setup_console_visibility(is_debug, logger)

        logger.info("===== 程序启动 =====")
        logger.info(f"启动参数: {sys.argv}")
        logger.info(f"开发模式: {'是' if is_debug else '否'}")

        root = tk.Tk()
        app = SkiHideApp(root, is_debug=is_debug, start_silent=is_silent)
        root.mainloop()

    except Exception as e:
        import traceback
        print("发生异常:")
        print(traceback.format_exc())
        input("按回车退出")

if __name__ == "__main__":
    main()

