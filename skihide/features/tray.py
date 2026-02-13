import threading
import traceback
import pystray
from pystray import MenuItem as item
from PIL import Image

from ..utils.paths import resource_path
from ..i18n import load_languages, set_language, get_available_languages, t


def start_tray(app, logger):
    """
    Start pystray icon in background thread.
    app must provide _restore_window() and quit_app().
    """
    try:
        # 使用 resource_path 获取托盘图标绝对路径
        icon_path = resource_path("icon.ico")

        # 打开图标，确保打包后可用
        try:
            image = Image.open(icon_path)
        except FileNotFoundError:
            logger.error(f"托盘图标文件未找到: {icon_path}")
            image = None  # 避免程序崩溃

        def on_show(icon, _item):
            # 在主线程显示窗口
            app.root.after(0, app._restore_window)

        def on_exit(icon, _item):
            # 在主线程退出应用
            app.root.after(0, app.quit_app)

        # 创建菜单项，使用翻译文本
        menu = pystray.Menu(
            item(" ", on_show, default=True, visible=False),
            item(t("tray.show"), on_show),
            item(t("tray.quit"), on_exit),
        )

        # 创建托盘图标对象
        tray_icon = pystray.Icon("SkiHide", image, "SkiHide", menu)

        # 托盘图标线程后台运行
        tray_thread = threading.Thread(target=tray_icon.run, daemon=True)
        tray_thread.start()

        return tray_icon, tray_thread

    except Exception:
        logger.error(f"托盘图标创建失败: {traceback.format_exc()}")
        raise
