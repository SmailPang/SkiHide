import os
import sys
import logging
import traceback
import ctypes
import threading
import requests
import json
import time
import webbrowser
from ctypes import cast, POINTER
from comtypes import CLSCTX_ALL
from pycaw.pycaw import AudioUtilities, IAudioEndpointVolume

# 定义Windows API常量
WM_APPCOMMAND = 0x319
APPCOMMAND_VOLUME_MUTE = 0x80000
APPCOMMAND_VOLUME_UP = 0xA0000
APPCOMMAND_VOLUME_DOWN = 0x90000
VK_VOLUME_MUTE = 0xAD
VK_VOLUME_UP = 0xAF
VK_VOLUME_DOWN = 0xAE

# 提权必须放最前面
if sys.platform == 'win32':
    try:
        if ctypes.windll.shell32.IsUserAnAdmin() == 0:
            ctypes.windll.shell32.ShellExecuteW(
                None, "runas", sys.executable, ' '.join(sys.argv), None, 1
            )
            sys.exit(0)
    except Exception as e:
        print("权限提升失败:", e)
        sys.exit(1)

# 设置日志系统
log_file = "log.txt"
if os.path.exists(log_file):
    with open(log_file, 'w'):
        pass

logging.basicConfig(
    level=logging.DEBUG,
    format='[%(asctime)s] %(message)s',
    datefmt='%Y-%m-%d %H:%M:%S',
    handlers=[
        logging.FileHandler(log_file, mode='a', encoding='utf-8'),
        logging.StreamHandler(sys.stdout)
    ]
)

import tkinter as tk
from tkinter import ttk, messagebox
import win32gui
import win32con
import win32process
import keyboard
from infi.systray import SysTrayIcon
from pynput import mouse

logger = logging.getLogger()


def resource_path(relative_path):
    """获取资源路径，适用于打包后的程序"""
    try:
        base_path = sys._MEIPASS
    except Exception:
        base_path = os.path.abspath(".")
    return os.path.join(base_path, relative_path)


class SkiHideApp:
    def __init__(self, root, is_debug=False):
        self.root = root
        self.root.title("SkiHide")
        self.current_version = "1.3.5"
        self.current_build = 26000
        self.is_debug = is_debug

        # 配置文件路径
        self.config_file = os.path.join(os.getcwd(), "config.json")

        # 初始化成员变量
        self.hotkey = None
        self.listener = None
        self.mouse_listener = None
        self.hidden_windows = {}
        self.tray_icon = None
        self.tray_thread = None
        self.recording_hotkey = False
        self.modifier_keys = set()
        self.mute_after_hide = False

        # 初始化音频控制
        self.is_muted = False
        self.saved_volume = None
        self.original_muted_state = False

        # 初始化pycaw音频端点
        try:
            device = AudioUtilities.GetSpeakers()
            self.volume = device.EndpointVolume
            self.is_muted = self.volume.GetMute()
            logger.info("pycaw音频控制初始化成功")
        except Exception as e:
            logger.error(f"pycaw音频控制初始化失败: {str(e)}")
            self.volume = None

        # 定义Windows API函数
        self.user32 = ctypes.windll.user32
        self.kernel32 = ctypes.windll.kernel32

        logger.info("音频控制初始化成功")

        # 加载图标
        try:
            icon_path = resource_path('icon.ico')
            self.root.iconbitmap(icon_path)
        except Exception as e:
            logger.warning(f"图标加载失败: {str(e)}")

        # ===== 首次启动隐私政策确认：不同意则退出 =====
        if not self.check_privacy_policy():
            try:
                self.root.destroy()
            except Exception:
                pass
            sys.exit(0)
        # =========================================

        # 创建界面元素
        self.create_widgets()
        self.setup_window_events()
        self.setup_tray_icon()
        self.populate_window_list()

        # 加载用户配置
        self.load_config()

        # 启动更新检查
        threading.Thread(target=self.check_for_updates, daemon=True).start()

        # keyboard 相关句柄（避免 unhook_all 误伤）
        self.hotkey_handle = None      # add_hotkey 返回的句柄
        self.record_hook = None        # keyboard.hook 返回的句柄
        self.was_listening_before_record = False

    # ==================== 首次启动隐私政策 ====================
    def read_config_safely(self):
        """安全读取配置：失败则返回空 dict"""
        try:
            if os.path.exists(self.config_file):
                with open(self.config_file, 'r', encoding='utf-8') as f:
                    return json.load(f) or {}
        except Exception:
            pass
        return {}

    def write_config_safely(self, config: dict):
        """安全写入配置：使用临时文件原子替换"""
        try:
            temp_file = self.config_file + ".tmp"
            with open(temp_file, 'w', encoding='utf-8') as f:
                json.dump(config, f, indent=2, ensure_ascii=False)
            os.replace(temp_file, self.config_file)
            return True
        except Exception as e:
            logger.error(f"写入配置失败: {str(e)}")
            return False

    def check_privacy_policy(self):
        """
        首次启动隐私政策确认：
        - 首次启动弹窗提示用户前往链接查看
        - 同意才可进入主界面
        - 不同意直接退出
        """
        try:
            config = self.read_config_safely()
            if config.get("privacy_accepted") is True:
                return True

            url = "https://flvsrttb.cn-nb1.rainapp.top/guide/privacy"
            msg = (
                "在使用 SkiHide 前，请您务必阅读并同意《隐私政策与免责说明》。\n\n"
                "请前往以下链接查看：\n"
                f"{url}\n\n"
                "点击“是”表示您已阅读并同意。\n"
                "点击“否”将退出程序（不同意则无法使用）。"
            )

            agreed = messagebox.askyesno("SkiHide - 隐私政策与免责说明", msg)
            if agreed:
                config["privacy_accepted"] = True
                self.write_config_safely(config)
                logger.info("用户已同意隐私政策与免责说明")
                return True
            else:
                logger.info("用户拒绝隐私政策与免责说明，程序退出")
                return False
        except Exception as e:
            logger.error(f"隐私政策检查异常: {traceback.format_exc()}")
            messagebox.showerror("SkiHide", "隐私政策确认流程发生错误，程序将退出。\n请查看 log.txt")
            return False

    # ==================== 菜单：问题反馈 ====================
    def open_feedback(self):
        """打开 GitHub Issues 反馈页（先提示可访问性）"""
        url = "https://github.com/Akttoer/SkiHide/issues"
        tip = (
            "问题反馈页面位于 GitHub。\n\n"
            "如遇到无法访问，请尝试使用加速器/代理等方式后再打开。\n\n"
            "点击“确定”将打开反馈页面。"
        )
        try:
            messagebox.showinfo("SkiHide - 问题反馈", tip)
            webbrowser.open(url, new=2)  # new=2 尽量用新标签页
        except Exception as e:
            messagebox.showerror("错误", f"打开反馈页面失败: {str(e)}")
            logger.error(f"打开反馈页面失败: {traceback.format_exc()}")

    # ==================== UI ====================
    def create_widgets(self):
        """创建界面组件"""
        # 创建菜单栏
        self.menu_bar = tk.Menu(self.root)
        self.root.config(menu=self.menu_bar)

        # 菜单项：设置
        self.menu_bar.add_command(label="设置", command=self.open_settings)

        # 菜单项：问题反馈（新增）
        self.menu_bar.add_command(label="问题反馈", command=self.open_feedback)

        # 如果是开发模式，添加开发者菜单
        if self.is_debug:
            dev_menu = tk.Menu(self.menu_bar, tearoff=0)
            self.menu_bar.add_cascade(label="开发者", menu=dev_menu)
            dev_menu.add_command(label="测试崩溃", command=self.test_crash)
            dev_menu.add_separator()
            dev_menu.add_command(label="刷新日志", command=self.refresh_log)
            dev_menu.add_command(label="查看系统信息", command=self.show_system_info)

        # 创建主框架
        main_frame = ttk.Frame(self.root, padding=10)
        main_frame.grid(row=0, column=0, sticky="nsew")

        # 窗口列表
        ttk.Label(main_frame, text="已打开的窗口:").grid(row=0, column=0, sticky="w")
        self.window_list = tk.Listbox(main_frame, width=50, height=15)
        self.window_list.grid(row=1, column=0, columnspan=2, pady=5)

        # 刷新按钮
        self.refresh_btn = ttk.Button(main_frame, text="刷新列表", command=self.populate_window_list)
        self.refresh_btn.grid(row=2, column=0, pady=5, sticky="ew")

        # 快捷键设置
        ttk.Label(main_frame, text="设置快捷键:").grid(row=3, column=0, sticky="w")
        self.hotkey_entry = ttk.Entry(main_frame, width=20)
        self.hotkey_entry.grid(row=3, column=1, padx=5, sticky="ew")
        self.hotkey_entry.bind("<FocusIn>", self.start_hotkey_recording)

        # 鼠标侧键复选框（默认不勾选）
        self.use_mouse_var = tk.BooleanVar(value=False)
        self.use_mouse_checkbox = ttk.Checkbutton(
            main_frame,
            text="使用鼠标侧键",
            variable=self.use_mouse_var,
            command=self.on_mouse_setting_change
        )
        self.use_mouse_checkbox.grid(row=4, column=0, sticky="w", pady=(5, 0))

        # 开始/停止监听按钮
        self.start_btn = ttk.Button(main_frame, text="开始监听", command=self.toggle_listener)
        self.start_btn.grid(row=5, column=0, columnspan=2, pady=10, sticky="ew")

    def test_crash(self):
        """测试崩溃功能"""
        logger.info("开始测试崩溃功能...")
        try:
            raise Exception("测试程序崩溃")
        except Exception as e:
            root = self.root
            error_msg = f"程序意外崩溃: {str(e)}"
            logger.critical(f"程序崩溃: {traceback.format_exc()}")

            try:
                system_info = get_system_info()
                logger.critical("===== 系统环境信息 =====")
                for key, value in system_info.items():
                    logger.critical(f"{key}: {value}")
            except Exception as sys_info_error:
                logger.critical(f"获取系统环境信息失败: {str(sys_info_error)}")

            messagebox.showerror(
                "程序崩溃",
                f"{error_msg}\n\n"
                f"程序意外崩溃，请将 log.txt 文件提交到反馈页面\n\n"
                f"崩溃时间: {time.strftime('%Y-%m-%d %H:%M:%S')}\n"
                f"错误信息: {str(e)}"
            )

            root.destroy()
            os._exit(1)

    def refresh_log(self):
        logger.info("刷新日志")
        messagebox.showinfo("提示", "日志已刷新")

    def show_system_info(self):
        logger.info("显示系统信息")
        try:
            system_info = get_system_info()
            info_str = "系统信息:\n\n"
            for key, value in system_info.items():
                info_str += f"{key}: {value}\n"
            messagebox.showinfo("系统信息", info_str)
        except Exception as e:
            messagebox.showerror("错误", f"获取系统信息失败: {str(e)}")

    def setup_window_events(self):
        self.root.protocol('WM_DELETE_WINDOW', self.on_close)
        self.root.bind("<Unmap>", self.on_minimize)

    def setup_tray_icon(self):
        """创建系统托盘图标"""
        try:
            menu_options = (
                ("显示主界面", None, self.restore_window),
                ("退出程序", None, self.quit_app)
            )
            icon_path = resource_path('icon.ico')
            self.tray_icon = SysTrayIcon(icon_path, "SkiHide", menu_options)
            self.tray_icon._double_click_left = self.restore_window
        except Exception as e:
            messagebox.showerror("错误", f"托盘图标创建失败: {str(e)}")
            logger.error(f"托盘图标创建失败: {traceback.format_exc()}")

    def populate_window_list(self):
        self.window_list.delete(0, tk.END)

        exclude_titles = {
            "Program Manager",
            "Windows 输入体验",
            "设置",
            "ASUSMascot",
            "AsHotplugCtrl",
            "NVIDIA GeForce Overlay",
            "NVIDIA Share",
            "NVIDIA Overlay",
            "Steam",
            "Discord",
            "Microsoft Text Input Application",
            "Windows Shell Experience Host",
            "SearchUI",
            "StartMenuExperienceHost",
            "SystemTray",
            "Desktop Window",
            "Armoury Crate"
        }

        current_pid = os.getpid()

        def enum_callback(hwnd, _):
            if not win32gui.IsWindow(hwnd):
                return
            if not win32gui.IsWindowVisible(hwnd):
                return

            title = win32gui.GetWindowText(hwnd)
            if not title.strip():
                return

            if title in exclude_titles:
                return

            try:
                _, pid = win32process.GetWindowThreadProcessId(hwnd)
                if pid == current_pid:
                    return
            except Exception as e:
                logger.debug(f"进程ID获取失败: {str(e)}")
                return

            try:
                style = win32gui.GetWindowLong(hwnd, win32con.GWL_STYLE)
                ex_style = win32gui.GetWindowLong(hwnd, win32con.GWL_EXSTYLE)

                if style & win32con.WS_CHILD:
                    return
                if ex_style & win32con.WS_EX_TOOLWINDOW:
                    return
                if ex_style & win32con.WS_EX_NOACTIVATE:
                    return

                rect = win32gui.GetWindowRect(hwnd)
                width = rect[2] - rect[0]
                height = rect[3] - rect[1]

                is_minimized = win32gui.GetWindowPlacement(hwnd)[1] == win32con.SW_SHOWMINIMIZED

                if not is_minimized and (width < 100 or height < 50):
                    return

                if not is_minimized and (rect[0] < -1000 or rect[1] < -1000 or rect[2] > 3000 or rect[3] > 3000):
                    return

            except Exception as e:
                logger.debug(f"窗口属性检查失败: {str(e)}")
                return

            try:
                parent_hwnd = win32gui.GetParent(hwnd)
                if parent_hwnd and parent_hwnd != 0:
                    return
            except Exception:
                pass

            self.window_list.insert(tk.END, (title, hwnd))

        win32gui.EnumWindows(enum_callback, None)
        logger.debug("已刷新窗口列表")

    def start_hotkey_recording(self, event=None):
        if self.recording_hotkey:
            return

        # 如果正在监听，先暂时停掉（避免录制时误触发）
        self.was_listening_before_record = bool(self.listener)
        if self.was_listening_before_record:
            # 只移除自己的热键，不要 unhook_all
            if self.hotkey_handle is not None:
                try:
                    keyboard.remove_hotkey(self.hotkey_handle)
                except Exception:
                    pass
                self.hotkey_handle = None

        self.recording_hotkey = True
        self.hotkey_entry.delete(0, tk.END)
        self.hotkey_entry.insert(0, "请按组合键...")
        self.hotkey_entry.after(100, self.listen_for_hotkey)

    def listen_for_hotkey(self):
        # 只卸载“录制用 hook”，不要 unhook_all
        if self.record_hook is not None:
            try:
                keyboard.unhook(self.record_hook)
            except Exception:
                pass
            self.record_hook = None

        recorded_keys = []
        self.modifier_keys.clear()

        def on_press(event):
            name = event.name

            # 修正 win 键命名（keyboard 更常用 win）
            if name in ['windows']:
                name = 'win'

            if name in ['ctrl', 'alt', 'shift', 'win']:
                self.modifier_keys.add(name)
                return

            # 规范化按键名
            if name == 'space':
                name = 'space'
            elif len(name) > 1 and name not in ['enter', 'tab', 'esc', 'backspace']:
                name = name.lower()

            if name not in recorded_keys:
                recorded_keys.append(name)

            combo = list(self.modifier_keys) + recorded_keys
            hotkey_str = '+'.join(combo)

            self.hotkey_entry.delete(0, tk.END)
            self.hotkey_entry.insert(0, hotkey_str)
            self.hotkey = hotkey_str
            logger.debug(f"已设置快捷键: {hotkey_str}")

            self.save_config()

            # 结束录制：卸载录制 hook
            if self.record_hook is not None:
                try:
                    keyboard.unhook(self.record_hook)
                except Exception:
                    pass
                self.record_hook = None

            self.modifier_keys.clear()
            self.recording_hotkey = False

            # 如果录制前在监听，录制完成后自动恢复监听（注册新热键）
            if self.was_listening_before_record:
                try:
                    self.hotkey_handle = keyboard.add_hotkey(self.hotkey, self.toggle_window)
                    logger.info(f"录制完成，已自动恢复监听并注册快捷键: {self.hotkey}")
                except Exception:
                    logger.error(f"自动恢复监听失败: {traceback.format_exc()}")

            self.was_listening_before_record = False
            self.root.focus()

        self.record_hook = keyboard.hook(on_press)

    def toggle_listener(self):
        if not self.hotkey and not self.use_mouse_var.get():
            messagebox.showerror("错误", "请先设置快捷键或启用鼠标侧键")
            return

        if not self.listener:
            try:
                # 启动监听：先清理旧的热键句柄（只清理自己的）
                if self.hotkey_handle is not None:
                    try:
                        keyboard.remove_hotkey(self.hotkey_handle)
                    except Exception:
                        pass
                    self.hotkey_handle = None

                # 注册快捷键
                if self.hotkey:
                    self.hotkey_handle = keyboard.add_hotkey(self.hotkey, self.toggle_window)
                    logger.info(f"已注册快捷键: {self.hotkey}")

                # 鼠标侧键
                if self.use_mouse_var.get():
                    self.start_mouse_listener()

                self.start_btn.config(text="停止监听")
                self.listener = True
            except Exception as e:
                messagebox.showerror("错误", f"快捷键注册失败: {str(e)}")
                logger.error(f"快捷键注册失败: {traceback.format_exc()}")
        else:
            # 停止监听：只移除自己的热键
            if self.hotkey_handle is not None:
                try:
                    keyboard.remove_hotkey(self.hotkey_handle)
                except Exception:
                    pass
                self.hotkey_handle = None

            # 停止鼠标监听
            if self.mouse_listener:
                try:
                    self.mouse_listener.stop()
                except Exception:
                    pass
                self.mouse_listener = None

            self.listener = None
            self.start_btn.config(text="开始监听")
            logger.info("已停止所有监听")

    def start_mouse_listener(self):
        def on_click(x, y, button, pressed):
            if pressed and str(button) in ['Button.x1', 'Button.x2']:
                self.toggle_window()

        self.mouse_listener = mouse.Listener(on_click=on_click)
        self.mouse_listener.daemon = True
        self.mouse_listener.start()
        logger.info("鼠标侧键监听已启用")

    def toggle_window(self):
        selection = self.window_list.curselection()
        if not selection:
            return

        title, hwnd = self.window_list.get(selection[0])
        if hwnd in self.hidden_windows:
            win32gui.ShowWindow(hwnd, win32con.SW_RESTORE)
            del self.hidden_windows[hwnd]
            logger.info(f"已显示窗口: {title}")

            if self.mute_after_hide and self.is_muted and self.volume and not self.original_muted_state:
                try:
                    self.volume.SetMute(False, None)
                    self.is_muted = False
                    logger.info("已恢复声音")
                except Exception as e:
                    logger.error(f"恢复声音失败: {str(e)}")
        else:
            win32gui.ShowWindow(hwnd, win32con.SW_HIDE)
            self.hidden_windows[hwnd] = title
            logger.info(f"已隐藏窗口: {title}")

            if self.mute_after_hide and self.volume:
                try:
                    current_mute = self.volume.GetMute()
                    current_volume = self.volume.GetMasterVolumeLevelScalar()

                    self.original_muted_state = current_mute

                    if not current_mute and current_volume > 0.0:
                        self.volume.SetMute(True, None)
                        self.is_muted = True
                        logger.info("已关闭声音")
                    else:
                        self.is_muted = current_mute
                        logger.info(f"保持当前静音状态: {current_mute}")
                except Exception as e:
                    logger.error(f"关闭声音失败: {str(e)}")

    def on_minimize(self, event):
        if self.root.state() == 'iconic':
            self.minimize_to_tray()

    def on_close(self):
        if messagebox.askyesno("退出程序", "确定要退出 SkiHide 吗？"):
            self.quit_app()
        else:
            self.minimize_to_tray()

    def minimize_to_tray(self):
        if self.tray_icon:
            self.root.withdraw()
            self.start_tray_thread()
            logger.info("主窗口已最小化到托盘")

    def start_tray_thread(self):
        if self.tray_icon and not self.tray_thread:
            self.tray_thread = threading.Thread(target=self.tray_icon.start, daemon=True)
            self.tray_thread.start()
            logger.debug("托盘线程已启动")

    def restore_window(self, systray=None):
        self.root.after(0, self._restore_window)

    def _restore_window(self):
        try:
            if not self.root.winfo_exists():
                self.root = tk.Tk()
                self.__init__(self.root, is_debug=self.is_debug)
            else:
                self.root.deiconify()
                self.root.lift()
                self.root.focus_force()
            logger.info("主窗口已恢复")
        except Exception as e:
            messagebox.showerror("错误", f"窗口恢复失败: {str(e)}")
            logger.error(f"窗口恢复失败: {traceback.format_exc()}")

    def on_mouse_setting_change(self):
        self.save_config()

    # ==================== 设置窗口 ====================
    def open_settings(self):
        self.settings_window = tk.Toplevel(self.root)
        self.settings_window.title("设置")
        self.settings_window.geometry("350x200")
        self.settings_window.resizable(False, False)

        self.settings_window.transient(self.root)
        self.settings_window.grab_set()

        self.settings_window.grid_rowconfigure(0, weight=1)
        self.settings_window.grid_rowconfigure(1, weight=0)
        self.settings_window.grid_columnconfigure(0, weight=1)

        settings_frame = ttk.Frame(self.settings_window, padding=20)
        settings_frame.grid(row=0, column=0, sticky="nsew")

        ttk.Label(settings_frame, text="隐藏后关闭声音:").grid(row=0, column=0, sticky="w", pady=10, padx=(0, 20))

        self.temp_mute_after_hide = self.mute_after_hide
        self.mute_after_hide_var = tk.BooleanVar(value=self.temp_mute_after_hide)

        self.mute_after_hide_switch = ttk.Checkbutton(
            settings_frame,
            variable=self.mute_after_hide_var
        )
        self.mute_after_hide_switch.grid(row=0, column=1, sticky="w")

        button_frame = ttk.Frame(self.settings_window, padding=(20, 10, 20, 20))
        button_frame.grid(row=1, column=0, sticky="se")

        ok_btn = ttk.Button(button_frame, text="确定", command=self.save_settings, width=8)
        ok_btn.grid(row=0, column=0, padx=5)

        apply_btn = ttk.Button(button_frame, text="应用", command=self.apply_settings, width=8)
        apply_btn.grid(row=0, column=1, padx=5)

        cancel_btn = ttk.Button(button_frame, text="取消", command=self.cancel_settings, width=8)
        cancel_btn.grid(row=0, column=2, padx=5)

        self.settings_window.protocol("WM_DELETE_WINDOW", self.cancel_settings)

    def save_settings(self):
        self.apply_settings()
        self.settings_window.destroy()

    def apply_settings(self):
        self.mute_after_hide = self.mute_after_hide_var.get()
        self.save_config()
        logger.info("设置已应用")

    def cancel_settings(self):
        self.settings_window.destroy()
        logger.info("设置已取消")

    def on_mute_setting_change(self):
        pass

    # ==================== 配置读写（不会丢字段） ====================
    def load_config(self):
        try:
            config = self.read_config_safely()

            if 'hotkey' in config and config['hotkey']:
                self.hotkey = config['hotkey']
                self.hotkey_entry.delete(0, tk.END)
                self.hotkey_entry.insert(0, self.hotkey)

            if 'use_mouse' in config:
                self.use_mouse_var.set(config['use_mouse'])

            self.mute_after_hide = config.get('mute_after_hide', False)

            logger.info("配置加载成功")
        except json.JSONDecodeError:
            logger.error("配置文件格式错误，使用默认配置")
            self.mute_after_hide = False
        except IOError as e:
            logger.error(f"配置文件读取失败: {str(e)}")
            self.mute_after_hide = False
        except Exception as e:
            logger.error(f"配置加载异常: {traceback.format_exc()}")
            self.mute_after_hide = False

    def save_config(self):
        """
        重要：读取旧配置 -> 合并更新 -> 再写回，避免丢 privacy_accepted 等字段
        """
        try:
            config = self.read_config_safely()
            config.update({
                'hotkey': self.hotkey,
                'use_mouse': self.use_mouse_var.get(),
                'mute_after_hide': getattr(self, 'mute_after_hide', False)
            })
            self.write_config_safely(config)
            logger.info("配置保存成功")
        except Exception as e:
            logger.error(f"配置保存异常: {traceback.format_exc()}")

    def quit_app(self, systray=None):
        try:
            self.save_config()

            if self.hotkey_handle is not None:
                try:
                    keyboard.remove_hotkey(self.hotkey_handle)
                except Exception:
                    pass
                self.hotkey_handle = None

            if self.record_hook is not None:
                try:
                    keyboard.unhook(self.record_hook)
                except Exception:
                    pass
                self.record_hook = None
            if self.mouse_listener:
                self.mouse_listener.stop()

            if self.tray_icon:
                pass

            self.root.destroy()
            logger.info("程序正常退出")
            sys.exit(0)
        except Exception as e:
            logger.critical(f"强制退出程序: {traceback.format_exc()}")
            os._exit(1)

    # ==================== 自动更新功能 ====================
    def check_for_updates(self):
        try:
            logger.info("正在检查更新...")
            update_url = "https://flvsrttb.cn-nb1.rainapp.top/v1"
            response = requests.get(update_url, timeout=5)
            response.raise_for_status()
            data = response.json()

            if self.compare_builds(data.get('build', 0)):
                new_version = data['version']
                new_build = data.get('build', 0)
                logger.info(f"发现新版本 {new_version}-build{new_build}，当前版本 {self.current_version}-build{self.current_build}")
                self.show_update_dialog(data)
            else:
                logger.info("当前已是最新版本")
        except requests.exceptions.RequestException as e:
            logger.warning(f"更新检查失败: {str(e)}")
        except Exception as e:
            logger.error(f"更新处理异常: {traceback.format_exc()}")

    def compare_builds(self, new_build):
        return int(new_build) > self.current_build

    def show_update_dialog(self, update_info):
        new_version = update_info['version']
        msg = f"发现新版本 {new_version}\n\n更新内容：\n{update_info.get('changelog', '')}"
        if messagebox.askyesno("发现更新", msg + "\n\n是否立即更新？"):
            self.start_download(update_info['download_url'])
        else:
            logger.info("用户取消更新")

    def start_download(self, url):
        self.update_window = tk.Toplevel(self.root)
        self.update_window.title("正在更新")
        self.update_window.geometry("300x120")
        self.update_window.resizable(False, False)

        ttk.Label(self.update_window, text="正在下载新版本...").pack(pady=10)
        self.progress = ttk.Progressbar(self.update_window, mode='determinate')
        self.progress.pack(fill=tk.X, padx=20, pady=5)

        threading.Thread(target=self.download_update, args=(url,), daemon=True).start()

    def download_update(self, url):
        try:
            temp_file = os.path.join(os.getcwd(), "update_temp.exe")
            with requests.get(url, stream=True, timeout=30) as r:
                r.raise_for_status()
                total_size = int(r.headers.get('content-length', 0))
                downloaded = 0

                with open(temp_file, 'wb') as f:
                    for chunk in r.iter_content(chunk_size=8192):
                        if chunk:
                            f.write(chunk)
                            downloaded += len(chunk)
                            progress = (downloaded / total_size) * 100 if total_size else 0
                            self.progress['value'] = progress
                            self.update_window.update()

            self.apply_update(temp_file)
        except Exception as e:
            messagebox.showerror("更新失败", f"下载失败: {str(e)}")
            self.update_window.destroy()
            logger.error(f"更新下载失败: {traceback.format_exc()}")

    def apply_update(self, temp_path):
        try:
            script = f"""@echo off
TIMEOUT /T 3 /NOBREAK >nul
taskkill /F /IM "{os.path.basename(sys.executable)}" >nul 2>&1
move /Y "{temp_path}" "{sys.executable}" >nul 2>&1
mshta vbscript:msgbox("更新已完成，请手动打开 SkiHide",0,"SkiHide")(window.close)
del "%~f0"
"""
            bat_path = os.path.join(os.getcwd(), "update_script.bat")
            with open(bat_path, "w", encoding="utf-8") as f:
                f.write(script)

            ctypes.windll.shell32.ShellExecuteW(None, "runas", bat_path, None, None, 0)
            self.quit_app()
        except Exception as e:
            messagebox.showerror("更新失败", f"应用更新时出错: {str(e)}")
            logger.error(f"更新应用失败: {traceback.format_exc()}")


def get_system_info():
    import platform
    import sys
    import os
    import psutil

    try:
        system = platform.system()
        release = platform.release()
        version = platform.version()
        machine = platform.machine()
        processor = platform.processor()

        windows_version = ""
        if system == "Windows":
            try:
                import win32api
                import winreg

                version_info = win32api.GetVersionEx()
                win_build = version_info[2]

                with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows NT\CurrentVersion") as key:
                    product_name = winreg.QueryValueEx(key, "ProductName")[0]
                    release_id = winreg.QueryValueEx(key, "ReleaseId")[0]
                    build_branch = winreg.QueryValueEx(key, "CurrentBuildBranch")[0]
                    display_version = winreg.QueryValueEx(key, "DisplayVersion")[0]
                    ubr = winreg.QueryValueEx(key, "UBR")[0]

                windows_version = f"{product_name} {display_version} {release_id} {win_build}.{ubr} {build_branch}"
            except Exception:
                windows_version = f"{system} {release} {version}"

        mem = psutil.virtual_memory()
        total_mem = mem.total / (1024**3)
        available_mem = mem.available / (1024**3)
        memory_info = f"{available_mem:.2f} GB / {total_mem:.2f} GB"

        disk = psutil.disk_usage('/')
        used_disk = disk.used / (1024**3)
        total_disk = disk.total / (1024**3)

        network_interfaces = [nic for nic, addrs in psutil.net_if_addrs().items()]

        script_path = os.path.abspath(__file__)
        exe_path = sys.executable

        system_info = {
            "系统版本": windows_version,
            "平台": platform.platform(),
            "系统": system,
            "版本号": release,
            "构建版本": version,
            "架构": machine,
            "处理器": processor,
            "Python版本": platform.python_version(),
            "Python构建": platform.python_build(),
            "Python编译器": platform.python_compiler(),
            "Python路径": exe_path,
            "软件路径": script_path,
            "工作目录": os.getcwd(),
            "CPU核心数": str(psutil.cpu_count()),
            "内存信息": memory_info,
            "可用内存": "%.2f GB" % available_mem,
            "总内存": "%.2f GB" % total_mem,
            "磁盘使用": "%.2f GB / %.2f GB" % (used_disk, total_disk),
            "网络接口": str(network_interfaces),
            "进程ID": str(os.getpid()),
            "父进程ID": str(os.getppid())
        }

        return system_info
    except Exception as e:
        system = platform.system()
        release = platform.release()
        version = platform.version()
        windows_version = f"{system} {release} {version}"

        return {
            "错误信息": str(e),
            "系统版本": windows_version,
            "平台": platform.platform(),
            "Python版本": platform.python_version(),
            "Python路径": sys.executable,
            "软件路径": os.path.abspath(__file__),
            "进程ID": str(os.getpid())
        }


def test_system_info():
    print("开始测试系统信息获取...")
    try:
        system_info = get_system_info()
        print("系统信息获取成功:")
        for key, value in system_info.items():
            print(f"{key}: {value}")
        print("\n所有系统信息值类型:")
        for key, value in system_info.items():
            print(f"{key}: {type(value).__name__}")
        print("\n系统信息测试通过!")
        return True
    except Exception as e:
        print(f"系统信息获取失败: {str(e)}")
        traceback.print_exc()
        return False


if __name__ == "__main__":
    is_debug = False
    for arg in sys.argv[1:]:
        if arg.upper() in ["-DEBUG", "--DEBUG", "/DEBUG"]:
            is_debug = True
            break

    for arg in sys.argv[1:]:
        if arg.upper() == "-TEST_SYSTEM_INFO":
            test_system_info()
            sys.exit(0)

    try:
        if sys.platform == 'win32':
            if is_debug:
                console_window = ctypes.windll.kernel32.GetConsoleWindow()
                if console_window:
                    ctypes.windll.user32.ShowWindow(console_window, 1)
                else:
                    ctypes.windll.kernel32.AllocConsole()
                    sys.stdout = open('CONOUT$', 'w', encoding='utf-8')
                    sys.stderr = open('CONERR$', 'w', encoding='utf-8')
                    console_handler = logging.StreamHandler(sys.stdout)
                    console_handler.setLevel(logging.DEBUG)
                    console_handler.setFormatter(logging.Formatter('[%(asctime)s] %(message)s', datefmt='%Y-%m-%d %H:%M:%S'))
                    logger.addHandler(console_handler)
                logger.info("开发模式启动 - 控制台窗口已显示")
            else:
                console_window = ctypes.windll.kernel32.GetConsoleWindow()
                if console_window:
                    ctypes.windll.user32.ShowWindow(console_window, 0)
                    logger.info("隐藏控制台窗口")

        logger.info("===== 程序启动 =====")
        logger.info(f"启动参数: {sys.argv}")
        logger.info(f"开发模式: {'是' if is_debug else '否'}")

        root = tk.Tk()
        app = SkiHideApp(root, is_debug)
        root.mainloop()

    except Exception as e:
        error_msg = f"程序意外崩溃: {str(e)}"
        logger.critical(f"程序崩溃: {traceback.format_exc()}")

        try:
            system_info = get_system_info()
            logger.critical("===== 系统环境信息 =====")
            for key, value in system_info.items():
                try:
                    log_value = str(value)
                    logger.critical(f"{key}: {log_value}")
                except Exception as format_error:
                    logger.critical(f"{key}: [无法格式化的值] - 错误: {str(format_error)}")
        except Exception as sys_info_error:
            logger.critical(f"获取系统环境信息失败: {str(sys_info_error)}")

        messagebox.showerror(
            "程序崩溃",
            f"{error_msg}\n\n"
            f"程序意外崩溃，请将 log.txt 文件提交到反馈页面\n\n"
            f"崩溃时间: {time.strftime('%Y-%m-%d %H:%M:%S')}\n"
            f"错误信息: {str(e)}"
        )

        os._exit(1)
