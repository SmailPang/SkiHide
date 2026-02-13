# -*- mode: python ; coding: utf-8 -*-

# 优化的打包配置，排除不必要的大型库

a = Analysis(
    ['main.py'],
    pathex=[],
    binaries=[],
    datas=[('icon.ico', '.', 'lang/*.json', 'lang')],
    hiddenimports=[],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[
        # 排除大型科学计算库
        'numpy', 'pandas', 'torch', 'tensorflow', 'scipy', 'sklearn',
        'matplotlib', 'seaborn', 'plotly', 'bokeh', 'holoviews',
        'sympy', 'mpmath', 'astropy',
        
        # 排除GUI库
        'pyqt5', 'pyside6', 'pyqtwebengine',
        
        # 排除开发工具库
        'pytest', 'pylint', 'mypy', 'flake8', 'black', 'isort',
        'jupyter', 'notebook', 'ipython', 'spyder',
        
        # 排除网络和异步库
        'aiohttp', 'asyncio', 'multiprocessing',
        'cryptography', 'bcrypt', 'paramiko',
        
        # 排除数据库库
        'sqlite3', 'mysql', 'postgresql', 'psycopg2', 'sqlalchemy',
        
        # 排除图像处理库
        'cv2', 'imageio', 'scikit-image',
        
        # 排除音频处理库（除了pycaw）
        'audioop', 'sounddevice', 'soundfile', 'librosa',
        
        # 排除其他大型库
        'scrapy',
        'conda', 'pip', 'setuptools',
        'win32com',
        'comtypes.client',
    ],
    noarchive=False,
    optimize=2,
)
pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name='main',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
    icon=['icon.ico'],
)
