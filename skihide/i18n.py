import os
import json
import locale

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LANG_DIR = os.path.join(BASE_DIR, "lang")

_current_lang = "zh_CN"
_translations = {}
_available = {}

def detect_system_language():
    try:
        sys_lang, _ = locale.getdefaultlocale()

        if not sys_lang:
            return "en_US"

        sys_lang = sys_lang.lower()

        # 中文环境
        if sys_lang.startswith("zh"):
            return "zh_CN"

        # 其他语言（例如 en_US / ja_JP）
        return sys_lang

    except Exception:
        return "en_US"


def load_languages():
    if not os.path.exists(LANG_DIR):
        return

    for file in os.listdir(LANG_DIR):
        if file.endswith(".json"):
            path = os.path.join(LANG_DIR, file)
            with open(path, "r", encoding="utf-8") as f:
                data = json.load(f)

            code = data["meta"]["code"]
            name = data["meta"]["name"]

            _available[code] = {
                "name": name,
                "file": path
            }


def set_language(lang):
    global _translations, _current_language

    path = os.path.join(LANG_DIR, f"{lang}.json")

    try:
        with open(path, "r", encoding="utf-8") as f:
            _translations = json.load(f)

        _current_language = lang

    except Exception:
        _translations = {}



def get_available_languages():
    languages = {}

    if not os.path.exists(LANG_DIR):
        return languages

    for filename in os.listdir(LANG_DIR):
        if filename.endswith(".json"):
            code = filename.replace(".json", "")
            path = os.path.join(LANG_DIR, filename)

            try:
                with open(path, "r", encoding="utf-8") as f:
                    data = json.load(f)

                name = data.get("meta", {}).get("name", code)

                languages[code] = {
                    "name": name
                }

            except Exception:
                continue

    return languages


def t(key, **kwargs):
    parts = key.split(".")
    value = _translations

    try:
        for p in parts:
            value = value[p]

        # 支持变量替换
        if kwargs and isinstance(value, str):
            return value.format(**kwargs)

        return value

    except Exception:
        return key
