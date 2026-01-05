import os
import sys
import json
import time
import ctypes
import shutil
import logging
import threading
import traceback
import webbrowser
import winreg
import requests

import tkinter as tk
from tkinter import ttk, messagebox

import win32gui
import win32con
import win32process
import keyboard
from pynput import mouse
from pycaw.pycaw import AudioUtilities

from .utils.paths import resource_path
from .utils.system_info import get_system_info
from .features.tray import start_tray
from .features.toolbox import clean_memory_working_set, clean_temp_folder

logger = logging.getLogger()

class SkiHideApp:
    def __init__(self, root, is_debug: bool = False, start_silent: bool = False):
        self.root = root
        self.root.title("SkiHide")
        self.current_version = "1.3.6"
        self.current_build = 26001
        self.is_debug = is_debug
        self.start_silent = start_silent

        self.config_file = os.path.join(os.getcwd(), "config.json")

        self.hotkey = None
        self.listener = None
        self.mouse_listener = None
        self.hidden_windows = {}

        self.tray_icon = None
        self.tray_thread = None

        self.recording_hotkey = False
        self.modifier_keys = set()
        self.mute_after_hide = False
        # Settings: autostart & scheduled memory cleaning
        self.autostart_enabled = False
        self.mem_clean_enabled = False
        self.mem_clean_value = 30
        self.mem_clean_unit = "分钟"
        self._mem_clean_after_id = None


        # Audio
        self.is_muted = False
        self.saved_volume = None
        self.original_muted_state = False
        self.muted_by_app = False

        try:
            device = AudioUtilities.GetSpeakers()
            self.volume = device.EndpointVolume
            self.is_muted = self.volume.GetMute()
            logger.info("pycaw音频控制初始化成功")
        except Exception as e:
            logger.error(f"pycaw音频控制初始化失败: {str(e)}")
            self.volume = None

        # icon
        try:
            icon_path = resource_path('icon.ico')
            self.root.iconbitmap(icon_path)
        except Exception as e:
            logger.warning(f"图标加载失败: {str(e)}")

        if not self.check_privacy_policy():
            try:
                self.root.destroy()
            except Exception:
                pass
            sys.exit(0)

        self.create_widgets()
        self.setup_window_events()

        # tray (double-click restore without bold menu item)
        try:
            self.tray_icon, self.tray_thread = start_tray(self, logger)
        except Exception as e:
            messagebox.showerror("错误", f"托盘图标创建失败: {str(e)}")

        self.populate_window_list()
        self.load_config()

        if getattr(self, "start_silent", False):
            try:
                self.minimize_to_tray()
            except Exception:
                pass

        threading.Thread(target=self.check_for_updates, daemon=True).start()

        self.hotkey_handle = None
        self.record_hook = None
        self.was_listening_before_record = False

    # -------- config safe io --------
    def read_config_safely(self):
        try:
            if os.path.exists(self.config_file):
                with open(self.config_file, 'r', encoding='utf-8') as f:
                    return json.load(f) or {}
        except Exception:
            pass
        return {}

    def write_config_safely(self, config: dict):
        try:
            temp_file = self.config_file + ".tmp"
            with open(temp_file, 'w', encoding='utf-8') as f:
                json.dump(config, f, indent=2, ensure_ascii=False)
            os.replace(temp_file, self.config_file)
            return True
        except Exception as e:
            logger.error(f"写入配置失败: {str(e)}")
            return False

    # -------- autostart (Windows) --------
    def _get_autostart_command(self) -> str:
        """Return command used for Windows autostart."""
        exe_path = sys.executable
        if " " in exe_path and not (exe_path.startswith('"') and exe_path.endswith('"')):
            exe_path = f'"{exe_path}"'

        if getattr(self, "silent_start_enabled", True):
            return f"{exe_path} --silent"

        return exe_path

    def set_autostart(self, enabled: bool):
        """Enable/disable autostart via HKCU\...\Run."""
        if sys.platform != "win32":
            raise RuntimeError("开机自启动仅支持 Windows")
        name = "SkiHide"
        key_path = r"Software\Microsoft\Windows\CurrentVersion\Run"
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, key_path, 0, winreg.KEY_SET_VALUE) as key:
            if enabled:
                winreg.SetValueEx(key, name, 0, winreg.REG_SZ, self._get_autostart_command())
            else:
                try:
                    winreg.DeleteValue(key, name)
                except FileNotFoundError:
                    pass

    # -------- scheduled memory cleaning --------
    def _mem_clean_interval_ms(self) -> int:
        value = int(getattr(self, "mem_clean_value", 30) or 30)
        value = max(1, min(999, value))
        unit = getattr(self, "mem_clean_unit", "分钟")
        if unit == "小时":
            return value * 60 * 60 * 1000
        return value * 60 * 1000

    def apply_memory_clean_scheduler(self):
        """Start/stop the periodic memory cleaning based on current settings."""
        # cancel existing schedule
        try:
            if getattr(self, "_mem_clean_after_id", None):
                self.root.after_cancel(self._mem_clean_after_id)
        except Exception:
            pass
        self._mem_clean_after_id = None

        if not getattr(self, "mem_clean_enabled", False):
            return

        interval = self._mem_clean_interval_ms()
        # Schedule first tick after interval (avoid doing heavy work immediately on enabling)
        self._mem_clean_after_id = self.root.after(interval, self._mem_clean_tick)

    def _mem_clean_tick(self):
        """One tick: run memory cleaning in background and reschedule."""
        def worker():
            try:
                cleaned, failed = clean_memory_working_set(logger)
                logger.info(f"定时清理内存完成：成功 {cleaned}，失败/跳过 {failed}")
            except Exception:
                logger.error(f"定时清理内存失败: {traceback.format_exc()}")

        try:
            threading.Thread(target=worker, daemon=True).start()
        except Exception:
            logger.error(f"启动清理线程失败: {traceback.format_exc()}")

        # reschedule
        try:
            interval = self._mem_clean_interval_ms()
            self._mem_clean_after_id = self.root.after(interval, self._mem_clean_tick)
        except Exception:
            self._mem_clean_after_id = None


    def check_privacy_policy(self):
        try:
            config = self.read_config_safely()
            if config.get("privacy_accepted") is True:
                return True

            url = "https://skihide.xyz/guide/privacy"
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
        except Exception:
            logger.error(f"隐私政策检查异常: {traceback.format_exc()}")
            messagebox.showerror("SkiHide", "隐私政策确认流程发生错误，程序将退出。\n请查看 log.txt")
            return False

    # -------- menu actions --------
    def open_feedback(self):
        url = "https://github.com/Akttoer/SkiHide/issues"
        tip = (
            "问题反馈页面位于 GitHub。\n\n"
            "如遇到无法访问，请尝试使用加速器/代理等方式后再打开。\n\n"
            "点击“确定”将打开反馈页面。"
        )
        try:
            messagebox.showinfo("SkiHide - 问题反馈", tip)
            webbrowser.open(url, new=2)
        except Exception as e:
            messagebox.showerror("错误", f"打开反馈页面失败: {str(e)}")
            logger.error(f"打开反馈页面失败: {traceback.format_exc()}")

    def open_toolbox(self):
        try:
            if hasattr(self, 'toolbox_window') and self.toolbox_window and self.toolbox_window.winfo_exists():
                self.toolbox_window.lift()
                self.toolbox_window.focus_force()
                return

            self.toolbox_window = tk.Toplevel(self.root)
            self.toolbox_window.title("百宝箱")
            self.toolbox_window.geometry("360x220")
            self.toolbox_window.resizable(False, False)
            self.toolbox_window.transient(self.root)

            frame = ttk.Frame(self.toolbox_window, padding=15)
            frame.pack(fill=tk.BOTH, expand=True)

            ttk.Label(frame, text="⚠️ 这些功能可能会影响系统稳定性，请谨慎使用。", wraplength=320).pack(anchor="w", pady=(0, 12))

            ttk.Button(frame, text="内存清理", command=self.confirm_and_clean_memory).pack(fill=tk.X, pady=5)
            ttk.Button(frame, text="清理缓存", command=self.confirm_and_clean_temp).pack(fill=tk.X, pady=5)
            ttk.Separator(frame).pack(fill=tk.X, pady=10)
            ttk.Button(frame, text="千万别点", command=self.danger_button_step1).pack(fill=tk.X, pady=5)

            self.toolbox_window.protocol("WM_DELETE_WINDOW", self.toolbox_window.destroy)
        except Exception:
            logger.error(f"打开百宝箱失败: {traceback.format_exc()}")
            messagebox.showerror("SkiHide - 百宝箱", "打开百宝箱失败，请查看 log.txt")

    def confirm_and_clean_memory(self):
        msg = (
            "即将执行【内存清理】\n\n"
            "这会尝试让系统回收部分进程的工作集（Working Set），可能带来：\n"
            "• 某些程序短暂卡顿/重新加载\n"
            "• 个别程序出现异常或崩溃（少见）\n\n"
            "是否继续？"
        )
        if not messagebox.askyesno("SkiHide - 内存清理", msg):
            return
        try:
            cleaned, failed = clean_memory_working_set(logger)
            messagebox.showinfo("SkiHide - 内存清理完成", f"成功: {cleaned}\n失败: {failed}")
        except Exception as e:
            logger.error(f"内存清理失败: {traceback.format_exc()}")
            messagebox.showerror("SkiHide - 内存清理", f"执行失败: {str(e)}")

    def confirm_and_clean_temp(self):
        msg = (
            "即将清理【系统临时缓存（%TEMP%）】\n\n"
            "可能带来：\n"
            "• 正在运行的软件临时文件被占用，部分文件无法删除（正常）\n"
            "• 少数软件可能需要重新生成缓存，首次启动变慢\n\n"
            "是否继续？"
        )
        if not messagebox.askyesno("SkiHide - 清理缓存", msg):
            return
        try:
            deleted_files, deleted_dirs, failed = clean_temp_folder(logger)
            messagebox.showinfo("SkiHide - 清理缓存完成", f"删除文件: {deleted_files}\n删除空目录: {deleted_dirs}\n失败/跳过: {failed}")
        except Exception as e:
            logger.error(f"清理缓存失败: {traceback.format_exc()}")
            messagebox.showerror("SkiHide - 清理缓存", f"执行失败: {str(e)}")

    def danger_button_step1(self):
        try:
            win = tk.Toplevel(self.root)
            win.title("千万别点")
            win.geometry("380x180")
            win.resizable(False, False)
            win.transient(self.root)
            win.grab_set()

            frame = ttk.Frame(win, padding=15)
            frame.pack(fill=tk.BOTH, expand=True)

            ttk.Label(frame, text="你确定要点击这个按钮吗？\n\n可能会发生意料之外的问题。", wraplength=340, justify="left").pack(anchor="w", pady=(0, 15))

            btn_frame = ttk.Frame(frame)
            btn_frame.pack(fill=tk.X)

            def cont():
                try:
                    win.destroy()
                except Exception:
                    pass
                messagebox.showinfo("继续", "你选择了继续……\n（其实什么都不会发生）")

            ttk.Button(btn_frame, text="继续", command=cont).pack(side=tk.LEFT, expand=True, fill=tk.X, padx=3)
            ttk.Button(btn_frame, text="继续", command=cont).pack(side=tk.LEFT, expand=True, fill=tk.X, padx=3)
            ttk.Button(btn_frame, text="继续", command=cont).pack(side=tk.LEFT, expand=True, fill=tk.X, padx=3)

            win.protocol("WM_DELETE_WINDOW", lambda: None)
        except Exception:
            logger.error(f"千万别点弹窗失败: {traceback.format_exc()}")
            messagebox.showerror("SkiHide", "弹窗失败，请查看 log.txt")

    # -------- UI --------
    def create_widgets(self):
        self.menu_bar = tk.Menu(self.root)
        self.root.config(menu=self.menu_bar)

        self.menu_bar.add_command(label="设置", command=self.open_settings)
        self.menu_bar.add_command(label="问题反馈", command=self.open_feedback)
        self.menu_bar.add_command(label="百宝箱", command=self.open_toolbox)

        if self.is_debug:
            dev_menu = tk.Menu(self.menu_bar, tearoff=0)
            self.menu_bar.add_cascade(label="开发者", menu=dev_menu)
            dev_menu.add_command(label="测试崩溃", command=self.test_crash)
            dev_menu.add_separator()
            dev_menu.add_command(label="刷新日志", command=self.refresh_log)
            dev_menu.add_command(label="查看系统信息", command=self.show_system_info)

        main_frame = ttk.Frame(self.root, padding=10)
        main_frame.grid(row=0, column=0, sticky="nsew")

        ttk.Label(main_frame, text="已打开的窗口:").grid(row=0, column=0, sticky="w")
        self.window_list = tk.Listbox(main_frame, width=50, height=15)
        self.window_list.grid(row=1, column=0, columnspan=2, pady=5)

        self.refresh_btn = ttk.Button(main_frame, text="刷新列表", command=self.populate_window_list)
        self.refresh_btn.grid(row=2, column=0, pady=5, sticky="ew")

        ttk.Label(main_frame, text="设置快捷键:").grid(row=3, column=0, sticky="w")
        self.hotkey_entry = ttk.Entry(main_frame, width=20)
        self.hotkey_entry.grid(row=3, column=1, padx=5, sticky="ew")
        self.hotkey_entry.bind("<FocusIn>", self.start_hotkey_recording)

        self.use_mouse_var = tk.BooleanVar(value=False)
        self.use_mouse_checkbox = ttk.Checkbutton(
            main_frame,
            text="使用鼠标侧键",
            variable=self.use_mouse_var,
            command=self.on_mouse_setting_change
        )
        self.use_mouse_checkbox.grid(row=4, column=0, sticky="w", pady=(5, 0))

        self.start_btn = ttk.Button(main_frame, text="开始监听", command=self.toggle_listener)
        self.start_btn.grid(row=5, column=0, columnspan=2, pady=10, sticky="ew")

    def setup_window_events(self):
        self.root.protocol('WM_DELETE_WINDOW', self.on_close)
        self.root.bind("<Unmap>", self.on_minimize)

    # -------- dev helpers --------
    def test_crash(self):
        logger.info("开始测试崩溃功能...")
        try:
            raise Exception("测试程序崩溃")
        except Exception as e:
            logger.critical(f"程序崩溃: {traceback.format_exc()}")
            try:
                system_info = get_system_info()
                for k, v in system_info.items():
                    logger.critical(f"{k}: {v}")
            except Exception:
                pass
            messagebox.showerror("程序崩溃", f"程序意外崩溃: {str(e)}\n\n请将 log.txt 提交到反馈页面")
            self.root.destroy()
            os._exit(1)

    def refresh_log(self):
        logger.info("刷新日志")
        messagebox.showinfo("提示", "日志已刷新")

    def show_system_info(self):
        logger.info("显示系统信息")
        try:
            system_info = get_system_info()
            info_str = "系统信息:\n\n" + "\n".join([f"{k}: {v}" for k, v in system_info.items()])
            messagebox.showinfo("系统信息", info_str)
        except Exception as e:
            messagebox.showerror("错误", f"获取系统信息失败: {str(e)}")

    # -------- window list --------
    def populate_window_list(self):
        self.window_list.delete(0, tk.END)

        exclude_titles = {
            "Program Manager","Windows 输入体验","设置","ASUSMascot","AsHotplugCtrl",
            "NVIDIA GeForce Overlay","NVIDIA Share","NVIDIA Overlay","Steam","Discord",
            "Microsoft Text Input Application","Windows Shell Experience Host","SearchUI",
            "StartMenuExperienceHost","SystemTray","Desktop Window","Armoury Crate"
        }

        current_pid = os.getpid()

        def enum_callback(hwnd, _):
            if not win32gui.IsWindow(hwnd): return
            if not win32gui.IsWindowVisible(hwnd): return
            title = win32gui.GetWindowText(hwnd)
            if not title.strip(): return
            if title in exclude_titles: return

            try:
                _, pid = win32process.GetWindowThreadProcessId(hwnd)
                if pid == current_pid: return
            except Exception:
                return

            try:
                style = win32gui.GetWindowLong(hwnd, win32con.GWL_STYLE)
                ex_style = win32gui.GetWindowLong(hwnd, win32con.GWL_EXSTYLE)
                if style & win32con.WS_CHILD: return
                if ex_style & win32con.WS_EX_TOOLWINDOW: return
                if ex_style & win32con.WS_EX_NOACTIVATE: return

                rect = win32gui.GetWindowRect(hwnd)
                width = rect[2] - rect[0]
                height = rect[3] - rect[1]
                is_minimized = win32gui.GetWindowPlacement(hwnd)[1] == win32con.SW_SHOWMINIMIZED
                if not is_minimized and (width < 100 or height < 50): return
            except Exception:
                return

            try:
                parent_hwnd = win32gui.GetParent(hwnd)
                if parent_hwnd and parent_hwnd != 0: return
            except Exception:
                pass

            self.window_list.insert(tk.END, (title, hwnd))

        win32gui.EnumWindows(enum_callback, None)

    # -------- hotkey recording --------
    def start_hotkey_recording(self, event=None):
        if self.recording_hotkey:
            return

        self.was_listening_before_record = bool(self.listener)
        if self.was_listening_before_record:
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
            if name in ['windows']:
                name = 'win'
            if name in ['ctrl', 'alt', 'shift', 'win']:
                self.modifier_keys.add(name)
                return

            if name not in recorded_keys:
                recorded_keys.append(name)

            combo = list(self.modifier_keys) + recorded_keys
            hotkey_str = '+'.join(combo)

            self.hotkey_entry.delete(0, tk.END)
            self.hotkey_entry.insert(0, hotkey_str)
            self.hotkey = hotkey_str
            self.save_config()

            if self.record_hook is not None:
                try:
                    keyboard.unhook(self.record_hook)
                except Exception:
                    pass
                self.record_hook = None

            self.modifier_keys.clear()
            self.recording_hotkey = False

            if self.was_listening_before_record:
                try:
                    self.hotkey_handle = keyboard.add_hotkey(self.hotkey, self.toggle_window)
                    logger.info(f"录制完成，已自动恢复监听并注册快捷键: {self.hotkey}")
                except Exception:
                    logger.error(f"自动恢复监听失败: {traceback.format_exc()}")

            self.was_listening_before_record = False
            self.root.focus()

        self.record_hook = keyboard.hook(on_press)

    # -------- listener switch --------
    def toggle_listener(self):
        if not self.hotkey and not self.use_mouse_var.get():
            messagebox.showerror("错误", "请先设置快捷键或启用鼠标侧键")
            return

        if not self.listener:
            try:
                if self.hotkey_handle is not None:
                    try: keyboard.remove_hotkey(self.hotkey_handle)
                    except Exception: pass
                    self.hotkey_handle = None

                if self.hotkey:
                    self.hotkey_handle = keyboard.add_hotkey(self.hotkey, self.toggle_window)

                if self.use_mouse_var.get():
                    self.start_mouse_listener()

                self.start_btn.config(text="停止监听")
                self.listener = True
            except Exception as e:
                messagebox.showerror("错误", f"快捷键注册失败: {str(e)}")
                logger.error(f"快捷键注册失败: {traceback.format_exc()}")
        else:
            if self.hotkey_handle is not None:
                try: keyboard.remove_hotkey(self.hotkey_handle)
                except Exception: pass
                self.hotkey_handle = None

            if self.mouse_listener:
                try: self.mouse_listener.stop()
                except Exception: pass
                self.mouse_listener = None

            self.listener = None
            self.start_btn.config(text="开始监听")

    def start_mouse_listener(self):
        def on_click(x, y, button, pressed):
            if pressed and str(button) in ['Button.x1', 'Button.x2']:
                self.toggle_window()
        self.mouse_listener = mouse.Listener(on_click=on_click)
        self.mouse_listener.daemon = True
        self.mouse_listener.start()

    # -------- window hide/restore --------
    def toggle_window(self):
        selection = self.window_list.curselection()
        if not selection:
            return
        title, hwnd = self.window_list.get(selection[0])
        if hwnd in self.hidden_windows:
            win32gui.ShowWindow(hwnd, win32con.SW_RESTORE)
            del self.hidden_windows[hwnd]

            # ✅ 如果已经没有任何隐藏窗口了，就恢复音量
            if len(self.hidden_windows) == 0:
                self._restore_system_audio_if_needed()
        else:
            win32gui.ShowWindow(hwnd, win32con.SW_HIDE)
            self.hidden_windows[hwnd] = title

            # ✅ 第一次进入“存在隐藏窗口”的状态时才触发静音
            if len(self.hidden_windows) == 1:
                self._mute_system_if_needed()

    # -------- minimize/close --------
    def on_minimize(self, event):
        if self.root.state() == 'iconic':
            self.minimize_to_tray()

    def on_close(self):
        if messagebox.askyesno("退出程序", "确定要退出 SkiHide 吗？"):
            self.quit_app()

    def minimize_to_tray(self):
        self.root.withdraw()

    def _restore_window(self):
        try:
            self.root.deiconify()
            self.root.lift()
            self.root.focus_force()
        except Exception as e:
            messagebox.showerror("错误", f"窗口恢复失败: {str(e)}")

    def on_mouse_setting_change(self):
        self.save_config()

    # -------- settings window (kept) --------
    def open_settings(self):
        self.settings_window = tk.Toplevel(self.root)
        self.settings_window.title("设置")
        self.settings_window.geometry("420x320")
        self.settings_window.resizable(False, False)

        self.settings_window.transient(self.root)
        self.settings_window.grab_set()

        settings_frame = ttk.Frame(self.settings_window, padding=20)
        settings_frame.grid(row=0, column=0, sticky="nsew")

        # ---- 1) Mute after hide ----
        ttk.Label(settings_frame, text="隐藏后关闭声音:").grid(row=0, column=0, sticky="w", pady=8, padx=(0, 20))

        self.temp_mute_after_hide = getattr(self, "mute_after_hide", False)
        self.mute_after_hide_var = tk.BooleanVar(value=self.temp_mute_after_hide)
        self.mute_after_hide_switch = ttk.Checkbutton(settings_frame, variable=self.mute_after_hide_var)
        self.mute_after_hide_switch.grid(row=0, column=1, sticky="w")

        # ---- 2) Autostart ----
        ttk.Label(settings_frame, text="开机自启动:").grid(row=1, column=0, sticky="w", pady=8, padx=(0, 20))

        self.autostart_var = tk.BooleanVar(value=getattr(self, "autostart_enabled", False))

        self.autostart_switch = ttk.Checkbutton(
            settings_frame,
            variable=self.autostart_var,
            command=self._on_autostart_toggle
        )
        self.autostart_switch.grid(row=1, column=1, sticky="w")

        ttk.Label(settings_frame, text="静默启动:").grid(row=2, column=0, sticky="w", pady=8, padx=(0, 20))
        self.silent_start_var = tk.BooleanVar(value=getattr(self, "silent_start_enabled", True))
        self.silent_start_chk = ttk.Checkbutton(settings_frame, variable=self.silent_start_var)
        self.silent_start_chk.grid(row=2, column=1, sticky="w")

        self._on_autostart_toggle()

        # ---- 3) Scheduled memory cleaning ----
        ttk.Label(settings_frame, text="定时清理内存:").grid(row=3, column=0, sticky="w", pady=8, padx=(0, 20))
        self.mem_clean_enabled_var = tk.BooleanVar(value=getattr(self, "mem_clean_enabled", False))
        self.mem_clean_enabled_chk = ttk.Checkbutton(
            settings_frame,
            variable=self.mem_clean_enabled_var,
            command=self._on_mem_clean_toggle
        )
        self.mem_clean_enabled_chk.grid(row=3, column=1, sticky="w")

        # interval row
        interval_frame = ttk.Frame(settings_frame)
        interval_frame.grid(row=4, column=0, columnspan=2, sticky="w", pady=(2, 10))

        ttk.Label(interval_frame, text="清理间隔:").grid(row=0, column=0, sticky="w", padx=(0, 10))

        self.mem_clean_value_var = tk.IntVar(value=int(getattr(self, "mem_clean_value", 30) or 30))
        self.mem_clean_value_spin = ttk.Spinbox(interval_frame, from_=1, to=999, textvariable=self.mem_clean_value_var, width=6)
        self.mem_clean_value_spin.grid(row=0, column=1, sticky="w", padx=(0, 10))

        self.mem_clean_unit_var = tk.StringVar(value=getattr(self, "mem_clean_unit", "分钟"))
        self.mem_clean_unit_combo = ttk.Combobox(interval_frame, textvariable=self.mem_clean_unit_var, values=["分钟", "小时"], width=6, state="readonly")
        self.mem_clean_unit_combo.grid(row=0, column=2, sticky="w")

        # apply initial enable/disable state
        self._on_mem_clean_toggle()

        # buttons
        button_frame = ttk.Frame(self.settings_window, padding=(20, 10, 20, 20))
        button_frame.grid(row=1, column=0, sticky="se")

        ttk.Button(button_frame, text="确定", command=self.save_settings, width=8).grid(row=0, column=0, padx=5)
        ttk.Button(button_frame, text="应用", command=self.apply_settings, width=8).grid(row=0, column=1, padx=5)
        ttk.Button(button_frame, text="取消", command=self.cancel_settings, width=8).grid(row=0, column=2, padx=5)

        self.settings_window.protocol("WM_DELETE_WINDOW", self.cancel_settings)


    
    def _on_mem_clean_toggle(self):
        """Enable/disable interval controls based on the checkbox."""
        try:
            enabled = bool(self.mem_clean_enabled_var.get())
        except Exception:
            enabled = False

        state = "normal" if enabled else "disabled"
        try:
            self.mem_clean_value_spin.configure(state=state)
        except Exception:
            pass
        try:
            self.mem_clean_unit_combo.configure(state=("readonly" if enabled else "disabled"))
        except Exception:
            pass

    def _on_autostart_toggle(self):
        """只有启用了开机自启动之后，静默启动才允许用户更改。"""
        try:
            enabled = bool(self.autostart_var.get())
        except Exception:
            enabled = False

        state = "normal" if enabled else "disabled"
        try:
            self.silent_start_chk.configure(state=state)
        except Exception:
            pass

    def save_settings(self):
        self.apply_settings()
        self.settings_window.destroy()

    def apply_settings(self):
        # 1) UI settings
        self.mute_after_hide = self.mute_after_hide_var.get()

        # 2) Autostart
        self.autostart_enabled = bool(getattr(self, "autostart_var", tk.BooleanVar(value=False)).get())
        self.silent_start_enabled = bool(getattr(self, "silent_start_var", tk.BooleanVar(value=True)).get())

        # 3) Scheduled memory cleaning
        self.mem_clean_enabled = bool(getattr(self, "mem_clean_enabled_var", tk.BooleanVar(value=False)).get())
        try:
            self.mem_clean_value = int(getattr(self, "mem_clean_value_var", tk.IntVar(value=30)).get())
        except Exception:
            self.mem_clean_value = 30
        self.mem_clean_value = max(1, min(999, int(self.mem_clean_value)))

        unit = str(getattr(self, "mem_clean_unit_var", tk.StringVar(value="分钟")).get() or "分钟")
        if unit not in ("分钟", "小时"):
            unit = "分钟"
        self.mem_clean_unit = unit

        # persist
        self.save_config()

        # apply autostart immediately
        try:
            self.set_autostart(self.autostart_enabled)
        except Exception as e:
            logger.error(f"开机自启动设置失败: {traceback.format_exc()}")
            messagebox.showwarning("SkiHide", f"开机自启动设置失败：{str(e)}")

        # restart scheduler
        try:
            self.apply_memory_clean_scheduler()
        except Exception:
            logger.error(f"定时清理内存调度失败: {traceback.format_exc()}")


    def cancel_settings(self):
        self.settings_window.destroy()

    # -------- config load/save --------
    def load_config(self):
        try:
            config = self.read_config_safely()
            if config.get('hotkey'):
                self.hotkey = config['hotkey']
                self.hotkey_entry.delete(0, tk.END)
                self.hotkey_entry.insert(0, self.hotkey)
            if 'use_mouse' in config:
                self.use_mouse_var.set(config['use_mouse'])
            self.mute_after_hide = config.get('mute_after_hide', False)
            self.autostart_enabled = bool(config.get('autostart_enabled', False))
            self.mem_clean_enabled = bool(config.get('mem_clean_enabled', False))
            self.silent_start_enabled = bool(config.get('silent_start_enabled', True))
            self.mem_clean_value = int(config.get('mem_clean_value', 30) or 30)
            self.mem_clean_unit = config.get('mem_clean_unit', "分钟") or "分钟"
            if self.mem_clean_unit not in ("分钟", "小时"):
                self.mem_clean_unit = "分钟"
            # apply scheduler on startup
            self.apply_memory_clean_scheduler()

        except Exception:
            logger.error(f"配置加载异常: {traceback.format_exc()}")

    def save_config(self):
        try:
            config = self.read_config_safely()
            config.update({
                'hotkey': self.hotkey,
                'use_mouse': self.use_mouse_var.get(),
                'mute_after_hide': getattr(self, 'mute_after_hide', False),
                'autostart_enabled': getattr(self, 'autostart_enabled', False),
                'mem_clean_enabled': getattr(self, 'mem_clean_enabled', False),
                'silent_start_enabled': getattr(self, 'silent_start_enabled', True),
                'mem_clean_value': getattr(self, 'mem_clean_value', 30),
                'mem_clean_unit': getattr(self, 'mem_clean_unit', '分钟')
            })
            self.write_config_safely(config)
        except Exception:
            logger.error(f"配置保存异常: {traceback.format_exc()}")

    # -------- exit --------
    def quit_app(self, *args, **kwargs):
        try:
            self.save_config()
            if self.hotkey_handle is not None:
                try: keyboard.remove_hotkey(self.hotkey_handle)
                except Exception: pass
                self.hotkey_handle = None

            if self.record_hook is not None:
                try: keyboard.unhook(self.record_hook)
                except Exception: pass
                self.record_hook = None

            if self.mouse_listener:
                try: self.mouse_listener.stop()
                except Exception: pass
                self.mouse_listener = None

            try:
                if self.tray_icon:
                    self.tray_icon.stop()
                    self.tray_icon = None
            except Exception:
                pass

            self.root.destroy()
            sys.exit(0)
        except Exception:
            os._exit(1)

    # -------- updates (kept same endpoints) --------
    def check_for_updates(self):
        try:
            update_url = "https://flvsrttb.cn-nb1.rainapp.top/v1"
            response = requests.get(update_url, timeout=5)
            response.raise_for_status()
            data = response.json()
            if int(data.get('build', 0)) > self.current_build:
                self.show_update_dialog(data)
        except Exception:
            logger.warning("更新检查失败")

    def show_update_dialog(self, update_info):
        new_version = update_info['version']
        msg = f"发现新版本 {new_version}\n\n更新内容：\n{update_info.get('changelog', '')}"
        if messagebox.askyesno("发现更新", msg + "\n\n是否立即更新？"):
            self.start_download(update_info['download_url'])

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
            try: self.update_window.destroy()
            except Exception: pass

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

    # -------- Audio --------

    def _mute_system_if_needed(self):
        """如果开启了隐藏后静音，且当前还没由本程序静音，则静音并记录状态"""
        if not getattr(self, "mute_after_hide", False):
            return
        if not self.volume:
            return

        try:
            # 只在第一次需要静音时记录
            if not self.muted_by_app:
                self.original_muted_state = bool(self.volume.GetMute())
                try:
                    self.saved_volume = float(self.volume.GetMasterVolumeLevelScalar())
                except Exception:
                    self.saved_volume = None

                # 如果用户本来没静音，我们才标记为“由本程序静音”
                if not self.original_muted_state:
                    self.volume.SetMute(1, None)
                    self.muted_by_app = True
        except Exception as e:
            logger.error(f"静音失败: {e}")

    def _restore_system_audio_if_needed(self):
        """当所有隐藏窗口都恢复后，如果之前是本程序静音的，则恢复"""
        if not self.volume:
            return

        try:
            if self.muted_by_app:
                # 恢复到用户原本状态
                self.volume.SetMute(0 if not self.original_muted_state else 1, None)
                if self.saved_volume is not None:
                    try:
                        self.volume.SetMasterVolumeLevelScalar(float(self.saved_volume), None)
                    except Exception:
                        pass
                self.muted_by_app = False
        except Exception as e:
            logger.error(f"恢复音量失败: {e}")